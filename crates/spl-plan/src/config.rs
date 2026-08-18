//! `S` and `Q` — the execution substrate and its architectural configuration.
//!
//! Stage 1 uses the smallest `S`/`Q` that makes the relation real rather than
//! imagined. `Q` binds the facts the plan actually depends on — the Plan IR
//! revision, the primitive-set revision, the integer types and the arithmetic
//! modes the substrate provides — and the VM refuses to run a plan that needs
//! something `Q` does not offer.
//!
//! No architectural adaptation is claimed, exercised or simulated here. `S` is
//! an identity for one prototype substrate and nothing reconfigures it.

use spl_core::value::Type;

/// The Plan IR revision this build implements.
pub const PLAN_IR_VERSION: u64 = 1;
/// The Core A primitive-set revision this build implements.
pub const PRIMITIVE_SET_VERSION: u64 = 1;

/// The arithmetic modes a configuration can offer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ArithmeticMode {
    Wrapping,
    Checked,
}

/// `Q` — the architectural configuration of the Plan VM.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlanVmConfiguration {
    pub plan_ir_version: u64,
    pub primitive_set_version: u64,
    /// Types the substrate provides. Sorted and deduplicated on construction so
    /// that two configurations offering the same capabilities have the same
    /// artifact bytes.
    pub supported_types: Vec<Type>,
    /// Arithmetic modes the substrate provides, likewise normalized.
    pub arithmetic_modes: Vec<ArithmeticMode>,
    /// Defensive execution budget, in executed instructions and terminators.
    pub step_limit: u64,
}

impl PlanVmConfiguration {
    /// Build a configuration, normalizing the capability lists.
    pub fn new(
        supported_types: impl IntoIterator<Item = Type>,
        arithmetic_modes: impl IntoIterator<Item = ArithmeticMode>,
        step_limit: u64,
    ) -> Self {
        let mut supported_types: Vec<Type> = supported_types.into_iter().collect();
        supported_types.sort_unstable();
        supported_types.dedup();
        let mut arithmetic_modes: Vec<ArithmeticMode> = arithmetic_modes.into_iter().collect();
        arithmetic_modes.sort_unstable();
        arithmetic_modes.dedup();
        PlanVmConfiguration {
            plan_ir_version: PLAN_IR_VERSION,
            primitive_set_version: PRIMITIVE_SET_VERSION,
            supported_types,
            arithmetic_modes,
            step_limit,
        }
    }

    /// The full Core A configuration used by the Stage-1 slice.
    pub fn stage1_full_core_a() -> Self {
        Self::new(
            [Type::Bool, Type::U8, Type::U32, Type::I32],
            [ArithmeticMode::Wrapping, ArithmeticMode::Checked],
            100_000,
        )
    }

    pub fn supports_type(&self, ty: Type) -> bool {
        self.supported_types.contains(&ty)
    }

    pub fn supports_mode(&self, mode: ArithmeticMode) -> bool {
        self.arithmetic_modes.contains(&mode)
    }
}

/// `S` — the identity of the execution substrate for one epoch.
///
/// Stage 1 does not model physical resources. This exists so that the later
/// `S`/`Q` distinction has a place to attach, and so that no code accidentally
/// treats `Q` as if it were the substrate itself.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SubstrateIdentity {
    pub label: String,
    pub epoch: u64,
}

impl SubstrateIdentity {
    pub fn new(label: impl Into<String>, epoch: u64) -> Self {
        SubstrateIdentity {
            label: label.into(),
            epoch,
        }
    }

    /// The single prototype substrate of the Stage-1 slice.
    pub fn stage1_prototype() -> Self {
        SubstrateIdentity::new("stage1-host-interpreter", 0)
    }
}
