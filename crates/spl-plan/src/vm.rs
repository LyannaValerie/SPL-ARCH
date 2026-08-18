//! The Plan VM.
//!
//! The VM interprets the Plan IR directly: it walks basic blocks, reads and
//! writes typed registers, and branches on boolean registers. It never builds a
//! `K` expression, never calls the `K` evaluator, and has no notion of a
//! "correct" answer to compare against. It executes what the plan says.
//!
//! Termination is structural, not policed: the validator rejects cyclic control
//! flow, so a validated plan always reaches a `Return` or an `Error`. The step
//! limit below is defence in depth for a future in which that stops being true,
//! and the Stage-1 tests assert it is never reached.

use spl_core::outcome::{ObservedOutcome, TrapKind};
use spl_core::prims;
use spl_core::value::{Type, Value};

use crate::config::{ArithmeticMode, PlanVmConfiguration, SubstrateIdentity};
use crate::ir::{Instruction, Terminator};
use crate::validate::ValidatedPlan;

/// Why the VM refused to run a plan.
///
/// None of these is an execution result. A plan that runs produces an
/// [`ObservedOutcome`]; a plan the substrate cannot host does not run at all.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum VmError {
    /// The configuration does not implement this Plan IR revision.
    PlanIrVersionMismatch { expected: u64, configured: u64 },
    /// The configuration does not implement this primitive-set revision.
    PrimitiveSetVersionMismatch { expected: u64, configured: u64 },
    /// The plan needs a type the configuration does not offer.
    UnsupportedType { ty: Type },
    /// The plan needs an arithmetic mode the configuration does not offer.
    UnsupportedArithmeticMode { mode: ArithmeticMode },
    /// The caller supplied the wrong number of inputs.
    InputArityMismatch { expected: usize, found: usize },
    /// The caller supplied an input of the wrong type.
    InputTypeMismatch {
        index: usize,
        expected: Type,
        found: Type,
    },
}

impl core::fmt::Display for VmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VmError::PlanIrVersionMismatch {
                expected,
                configured,
            } => write!(
                f,
                "plan needs Plan IR version {expected}, configuration provides {configured}"
            ),
            VmError::PrimitiveSetVersionMismatch {
                expected,
                configured,
            } => write!(
                f,
                "plan needs primitive set version {expected}, configuration provides {configured}"
            ),
            VmError::UnsupportedType { ty } => {
                write!(f, "the configuration does not provide {ty}")
            }
            VmError::UnsupportedArithmeticMode { mode } => {
                write!(f, "the configuration does not provide {mode:?} arithmetic")
            }
            VmError::InputArityMismatch { expected, found } => {
                write!(f, "plan expects {expected} input(s), got {found}")
            }
            VmError::InputTypeMismatch {
                index,
                expected,
                found,
            } => write!(f, "input {index} expects {expected}, got {found}"),
        }
    }
}

/// The Stage-1 Plan VM, bound to one substrate identity and one configuration.
#[derive(Clone, Debug)]
pub struct PlanVm {
    substrate: SubstrateIdentity,
    config: PlanVmConfiguration,
}

impl PlanVm {
    pub fn new(substrate: SubstrateIdentity, config: PlanVmConfiguration) -> Self {
        PlanVm { substrate, config }
    }

    /// A VM on the Stage-1 prototype substrate with the full Core A configuration.
    pub fn stage1() -> Self {
        PlanVm::new(
            SubstrateIdentity::stage1_prototype(),
            PlanVmConfiguration::stage1_full_core_a(),
        )
    }

    pub fn substrate(&self) -> &SubstrateIdentity {
        &self.substrate
    }

    pub fn config(&self) -> &PlanVmConfiguration {
        &self.config
    }

    /// Check that this configuration can host this plan at all.
    pub fn check_compatible(&self, plan: &ValidatedPlan) -> Result<(), VmError> {
        if self.config.plan_ir_version != crate::config::PLAN_IR_VERSION {
            return Err(VmError::PlanIrVersionMismatch {
                expected: crate::config::PLAN_IR_VERSION,
                configured: self.config.plan_ir_version,
            });
        }
        if self.config.primitive_set_version != crate::config::PRIMITIVE_SET_VERSION {
            return Err(VmError::PrimitiveSetVersionMismatch {
                expected: crate::config::PRIMITIVE_SET_VERSION,
                configured: self.config.primitive_set_version,
            });
        }
        for ty in plan.required_types() {
            if !self.config.supports_type(ty) {
                return Err(VmError::UnsupportedType { ty });
            }
        }
        for block in &plan.plan().blocks {
            for instruction in &block.instructions {
                let mode = match instruction {
                    Instruction::Wrapping { .. } => Some(ArithmeticMode::Wrapping),
                    Instruction::Checked { .. } => Some(ArithmeticMode::Checked),
                    _ => None,
                };
                if let Some(mode) = mode {
                    if !self.config.supports_mode(mode) {
                        return Err(VmError::UnsupportedArithmeticMode { mode });
                    }
                }
            }
        }
        Ok(())
    }

    /// Run a validated plan.
    ///
    /// The outer `Result` carries refusals to run. Everything the plan itself
    /// produces — including a semantic error — is in the [`ObservedOutcome`].
    pub fn run(&self, plan: &ValidatedPlan, inputs: &[Value]) -> Result<ObservedOutcome, VmError> {
        self.check_compatible(plan)?;
        let declared = &plan.plan().params;
        if inputs.len() != declared.len() {
            return Err(VmError::InputArityMismatch {
                expected: declared.len(),
                found: inputs.len(),
            });
        }
        for (index, (expected, supplied)) in declared.iter().zip(inputs).enumerate() {
            if *expected != supplied.ty() {
                return Err(VmError::InputTypeMismatch {
                    index,
                    expected: *expected,
                    found: supplied.ty(),
                });
            }
        }
        Ok(self.execute(plan, inputs))
    }

    fn execute(&self, plan: &ValidatedPlan, inputs: &[Value]) -> ObservedOutcome {
        let mut registers: Vec<Option<Value>> = vec![None; plan.register_slots()];
        let mut block_index = 0usize;
        let mut steps = 0u64;

        loop {
            let block = match plan.plan().blocks.get(block_index) {
                Some(block) => block,
                // Unreachable for a validated plan: block targets were checked.
                None => return ObservedOutcome::Stuck,
            };

            for instruction in &block.instructions {
                steps += 1;
                if steps > self.config.step_limit {
                    return ObservedOutcome::Trap(TrapKind::StepLimitExceeded);
                }
                match self.step(instruction, inputs, &mut registers) {
                    Ok(()) => {}
                    Err(stuck) => return stuck,
                }
            }

            steps += 1;
            if steps > self.config.step_limit {
                return ObservedOutcome::Trap(TrapKind::StepLimitExceeded);
            }

            match &block.terminator {
                Terminator::Jump { target } => block_index = target.0 as usize,
                Terminator::Branch {
                    cond,
                    if_true,
                    if_false,
                } => match read(&registers, cond.0) {
                    Some(Value::Bool(true)) => block_index = if_true.0 as usize,
                    Some(Value::Bool(false)) => block_index = if_false.0 as usize,
                    // Unreachable for a validated plan: the condition was typed.
                    _ => return ObservedOutcome::Stuck,
                },
                Terminator::Return { value } => {
                    return match read(&registers, value.0) {
                        Some(value) => ObservedOutcome::Return(value),
                        None => ObservedOutcome::Stuck,
                    }
                }
                Terminator::Error { kind } => return ObservedOutcome::SemanticError(*kind),
            }
        }
    }

    /// Execute one instruction. `Err` carries the observed outcome to stop with,
    /// which for a validated plan is unreachable.
    fn step(
        &self,
        instruction: &Instruction,
        inputs: &[Value],
        registers: &mut [Option<Value>],
    ) -> Result<(), ObservedOutcome> {
        match instruction {
            Instruction::Input { dst, index } => {
                let value = *inputs.get(*index as usize).ok_or(ObservedOutcome::Stuck)?;
                write(registers, dst.0, value)
            }
            Instruction::Const { dst, value } => write(registers, dst.0, *value),
            Instruction::Compare { dst, op, lhs, rhs } => {
                let lhs = read(registers, lhs.0).ok_or(ObservedOutcome::Stuck)?;
                let rhs = read(registers, rhs.0).ok_or(ObservedOutcome::Stuck)?;
                let result = prims::compare(*op, lhs, rhs).map_err(|_| ObservedOutcome::Stuck)?;
                write(registers, dst.0, Value::Bool(result))
            }
            Instruction::Wrapping { dst, op, lhs, rhs } => {
                let lhs = read(registers, lhs.0).ok_or(ObservedOutcome::Stuck)?;
                let rhs = read(registers, rhs.0).ok_or(ObservedOutcome::Stuck)?;
                let result = prims::wrapping(*op, lhs, rhs).map_err(|_| ObservedOutcome::Stuck)?;
                write(registers, dst.0, result)
            }
            Instruction::Checked {
                value,
                overflow,
                op,
                lhs,
                rhs,
            } => {
                let lhs_value = read(registers, lhs.0).ok_or(ObservedOutcome::Stuck)?;
                let rhs_value = read(registers, rhs.0).ok_or(ObservedOutcome::Stuck)?;
                let checked = prims::checked(*op, lhs_value, rhs_value)
                    .map_err(|_| ObservedOutcome::Stuck)?;
                // Both results are written unconditionally. Deciding what an
                // overflow *means* is the plan's job, expressed as a branch on
                // the flag; the VM has no opinion about it.
                write(registers, value.0, checked.wrapped)?;
                write(registers, overflow.0, Value::Bool(checked.overflow))
            }
        }
    }
}

fn read(registers: &[Option<Value>], index: u32) -> Option<Value> {
    registers.get(index as usize).copied().flatten()
}

fn write(registers: &mut [Option<Value>], index: u32, value: Value) -> Result<(), ObservedOutcome> {
    match registers.get_mut(index as usize) {
        Some(slot) => {
            *slot = Some(value);
            Ok(())
        }
        // Unreachable for a validated plan: the slot count comes from the
        // validator's own register map.
        None => Err(ObservedOutcome::Stuck),
    }
}
