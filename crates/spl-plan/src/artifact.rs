//! Artifact forms of `P` and `Q`.
//!
//! These implement the same [`Artifact`] contract as the `K` artifact: a
//! `[schema_tag, schema_version, body]` envelope, canonical bytes, and a
//! content identity over those exact bytes. The schema tags differ, so a `P`
//! artifact can never decode as a `K` artifact even if the bodies were
//! byte-identical.
//!
//! The Plan body is encoded here, not derived from any `K` encoding. The two
//! encodings share only the leaf vocabulary — how a `u8` literal or a
//! comparison operator is written down — which is a primitive type definition,
//! not a representation of either object.

use ciborium::value::Value as CborValue;
use spl_core::artifact::{
    decode_arith_op, decode_compare_op, decode_semantic_error, decode_type, decode_value,
    encode_arith_op, encode_compare_op, encode_semantic_error, encode_type, encode_value,
    expect_array, expect_list, expect_text, expect_uint, Artifact, ArtifactError, SCHEMA_TAG_PLAN,
    SCHEMA_TAG_Q,
};
use spl_core::canonical;
use spl_core::value::Type;

use crate::config::{ArithmeticMode, PlanVmConfiguration};
use crate::ir::{Block, BlockId, Instruction, Plan, Reg, Terminator, MAX_BLOCKS, MAX_REGISTERS};

fn structure(detail: impl Into<String>) -> ArtifactError {
    ArtifactError::Structure(detail.into())
}

fn encode_reg(reg: Reg) -> CborValue {
    canonical::uint(u64::from(reg.0))
}

fn decode_reg(value: &CborValue) -> Result<Reg, ArtifactError> {
    let raw = expect_uint(value, "register")?;
    if raw >= u64::from(MAX_REGISTERS) {
        return Err(structure(format!(
            "register r{raw} exceeds the plan register limit of {MAX_REGISTERS}"
        )));
    }
    Ok(Reg(raw as u32))
}

fn encode_block_id(id: BlockId) -> CborValue {
    canonical::uint(u64::from(id.0))
}

fn decode_block_id(value: &CborValue) -> Result<BlockId, ArtifactError> {
    let raw = expect_uint(value, "block target")?;
    if raw >= u64::from(MAX_BLOCKS) {
        return Err(structure(format!(
            "block target {raw} exceeds the plan block limit of {MAX_BLOCKS}"
        )));
    }
    Ok(BlockId(raw as u32))
}

fn encode_instruction(instruction: &Instruction) -> CborValue {
    match instruction {
        Instruction::Input { dst, index } => canonical::array(vec![
            canonical::uint(0),
            encode_reg(*dst),
            canonical::uint(u64::from(*index)),
        ]),
        Instruction::Const { dst, value } => canonical::array(vec![
            canonical::uint(1),
            encode_reg(*dst),
            encode_value(*value),
        ]),
        Instruction::Compare { dst, op, lhs, rhs } => canonical::array(vec![
            canonical::uint(2),
            encode_reg(*dst),
            encode_compare_op(*op),
            encode_reg(*lhs),
            encode_reg(*rhs),
        ]),
        Instruction::Wrapping { dst, op, lhs, rhs } => canonical::array(vec![
            canonical::uint(3),
            encode_reg(*dst),
            encode_arith_op(*op),
            encode_reg(*lhs),
            encode_reg(*rhs),
        ]),
        Instruction::Checked {
            value,
            overflow,
            op,
            lhs,
            rhs,
        } => canonical::array(vec![
            canonical::uint(4),
            encode_reg(*value),
            encode_reg(*overflow),
            encode_arith_op(*op),
            encode_reg(*lhs),
            encode_reg(*rhs),
        ]),
    }
}

fn decode_instruction(value: &CborValue) -> Result<Instruction, ArtifactError> {
    let items = expect_list(value, "instruction")?;
    let tag = items
        .first()
        .ok_or_else(|| structure("instruction must not be empty"))
        .and_then(|item| expect_uint(item, "instruction tag"))?;
    match (tag, items.len()) {
        (0, 3) => {
            let raw = expect_uint(&items[2], "input index")?;
            let index = u32::try_from(raw)
                .map_err(|_| structure(format!("input index {raw} is out of range")))?;
            Ok(Instruction::Input {
                dst: decode_reg(&items[1])?,
                index,
            })
        }
        (1, 3) => Ok(Instruction::Const {
            dst: decode_reg(&items[1])?,
            value: decode_value(&items[2])?,
        }),
        (2, 5) => Ok(Instruction::Compare {
            dst: decode_reg(&items[1])?,
            op: decode_compare_op(&items[2])?,
            lhs: decode_reg(&items[3])?,
            rhs: decode_reg(&items[4])?,
        }),
        (3, 5) => Ok(Instruction::Wrapping {
            dst: decode_reg(&items[1])?,
            op: decode_arith_op(&items[2])?,
            lhs: decode_reg(&items[3])?,
            rhs: decode_reg(&items[4])?,
        }),
        (4, 6) => Ok(Instruction::Checked {
            value: decode_reg(&items[1])?,
            overflow: decode_reg(&items[2])?,
            op: decode_arith_op(&items[3])?,
            lhs: decode_reg(&items[4])?,
            rhs: decode_reg(&items[5])?,
        }),
        (0..=4, len) => Err(structure(format!(
            "instruction tag {tag} does not take {len} field(s)"
        ))),
        _ => Err(ArtifactError::UnknownTag {
            position: "instruction",
            tag,
        }),
    }
}

fn encode_terminator(terminator: &Terminator) -> CborValue {
    match terminator {
        Terminator::Jump { target } => {
            canonical::array(vec![canonical::uint(0), encode_block_id(*target)])
        }
        Terminator::Branch {
            cond,
            if_true,
            if_false,
        } => canonical::array(vec![
            canonical::uint(1),
            encode_reg(*cond),
            encode_block_id(*if_true),
            encode_block_id(*if_false),
        ]),
        Terminator::Return { value } => {
            canonical::array(vec![canonical::uint(2), encode_reg(*value)])
        }
        Terminator::Error { kind } => {
            canonical::array(vec![canonical::uint(3), encode_semantic_error(*kind)])
        }
    }
}

fn decode_terminator(value: &CborValue) -> Result<Terminator, ArtifactError> {
    let items = expect_list(value, "terminator")?;
    let tag = items
        .first()
        .ok_or_else(|| structure("terminator must not be empty"))
        .and_then(|item| expect_uint(item, "terminator tag"))?;
    match (tag, items.len()) {
        (0, 2) => Ok(Terminator::Jump {
            target: decode_block_id(&items[1])?,
        }),
        (1, 4) => Ok(Terminator::Branch {
            cond: decode_reg(&items[1])?,
            if_true: decode_block_id(&items[2])?,
            if_false: decode_block_id(&items[3])?,
        }),
        (2, 2) => Ok(Terminator::Return {
            value: decode_reg(&items[1])?,
        }),
        (3, 2) => Ok(Terminator::Error {
            kind: decode_semantic_error(&items[1])?,
        }),
        (0..=3, len) => Err(structure(format!(
            "terminator tag {tag} does not take {len} field(s)"
        ))),
        _ => Err(ArtifactError::UnknownTag {
            position: "terminator",
            tag,
        }),
    }
}

/// The standalone artifact form of an execution plan.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlanArtifact {
    pub plan: Plan,
}

impl PlanArtifact {
    pub fn new(plan: Plan) -> Self {
        PlanArtifact { plan }
    }
}

impl Artifact for PlanArtifact {
    const SCHEMA_TAG: u64 = SCHEMA_TAG_PLAN;
    const SCHEMA_VERSION: u64 = 1;

    fn to_body(&self) -> CborValue {
        let blocks = self
            .plan
            .blocks
            .iter()
            .map(|block| {
                canonical::array(vec![
                    canonical::array(block.instructions.iter().map(encode_instruction).collect()),
                    encode_terminator(&block.terminator),
                ])
            })
            .collect();
        canonical::array(vec![
            canonical::text(&self.plan.name),
            canonical::array(self.plan.params.iter().copied().map(encode_type).collect()),
            encode_type(self.plan.result),
            canonical::array(blocks),
        ])
    }

    fn from_body(body: &CborValue) -> Result<Self, ArtifactError> {
        let items = expect_array(body, 4, "plan")?;
        let name = expect_text(&items[0], "plan name")?;
        let params = expect_list(&items[1], "parameter list")?
            .iter()
            .map(decode_type)
            .collect::<Result<Vec<_>, _>>()?;
        let result = decode_type(&items[2])?;
        let raw_blocks = expect_list(&items[3], "block list")?;
        if raw_blocks.len() > MAX_BLOCKS as usize {
            return Err(structure(format!(
                "plan has {} blocks; the limit is {MAX_BLOCKS}",
                raw_blocks.len()
            )));
        }
        let blocks = raw_blocks
            .iter()
            .map(|raw| {
                let fields = expect_array(raw, 2, "block")?;
                let instructions = expect_list(&fields[0], "instruction list")?
                    .iter()
                    .map(decode_instruction)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Block {
                    instructions,
                    terminator: decode_terminator(&fields[1])?,
                })
            })
            .collect::<Result<Vec<_>, ArtifactError>>()?;
        Ok(PlanArtifact {
            plan: Plan {
                name,
                params,
                result,
                blocks,
            },
        })
    }
}

fn encode_arithmetic_mode(mode: ArithmeticMode) -> CborValue {
    canonical::uint(match mode {
        ArithmeticMode::Wrapping => 0,
        ArithmeticMode::Checked => 1,
    })
}

fn decode_arithmetic_mode(value: &CborValue) -> Result<ArithmeticMode, ArtifactError> {
    match expect_uint(value, "arithmetic mode")? {
        0 => Ok(ArithmeticMode::Wrapping),
        1 => Ok(ArithmeticMode::Checked),
        tag => Err(ArtifactError::UnknownTag {
            position: "arithmetic mode",
            tag,
        }),
    }
}

/// The standalone artifact form of the architectural configuration `Q`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QArtifact {
    pub config: PlanVmConfiguration,
}

impl QArtifact {
    pub fn new(config: PlanVmConfiguration) -> Self {
        QArtifact { config }
    }
}

impl Artifact for QArtifact {
    const SCHEMA_TAG: u64 = SCHEMA_TAG_Q;
    const SCHEMA_VERSION: u64 = 1;

    fn to_body(&self) -> CborValue {
        canonical::array(vec![
            canonical::uint(self.config.plan_ir_version),
            canonical::uint(self.config.primitive_set_version),
            canonical::array(
                self.config
                    .supported_types
                    .iter()
                    .copied()
                    .map(encode_type)
                    .collect(),
            ),
            canonical::array(
                self.config
                    .arithmetic_modes
                    .iter()
                    .copied()
                    .map(encode_arithmetic_mode)
                    .collect(),
            ),
            canonical::uint(self.config.step_limit),
        ])
    }

    fn from_body(body: &CborValue) -> Result<Self, ArtifactError> {
        let items = expect_array(body, 5, "configuration")?;
        let plan_ir_version = expect_uint(&items[0], "Plan IR version")?;
        let primitive_set_version = expect_uint(&items[1], "primitive set version")?;
        let supported_types = expect_list(&items[2], "supported type list")?
            .iter()
            .map(decode_type)
            .collect::<Result<Vec<Type>, _>>()?;
        let arithmetic_modes = expect_list(&items[3], "arithmetic mode list")?
            .iter()
            .map(decode_arithmetic_mode)
            .collect::<Result<Vec<_>, _>>()?;
        let step_limit = expect_uint(&items[4], "step limit")?;

        // The capability lists are normalized on construction, so a body whose
        // lists are unsorted or repeat an entry is a different byte string for
        // the same configuration. Rejecting it here keeps one configuration to
        // one identity.
        let mut sorted_types = supported_types.clone();
        sorted_types.sort_unstable();
        sorted_types.dedup();
        if sorted_types != supported_types {
            return Err(structure(
                "supported type list must be sorted and free of duplicates",
            ));
        }
        let mut sorted_modes = arithmetic_modes.clone();
        sorted_modes.sort_unstable();
        sorted_modes.dedup();
        if sorted_modes != arithmetic_modes {
            return Err(structure(
                "arithmetic mode list must be sorted and free of duplicates",
            ));
        }

        Ok(QArtifact {
            config: PlanVmConfiguration {
                plan_ir_version,
                primitive_set_version,
                supported_types,
                arithmetic_modes,
                step_limit,
            },
        })
    }
}
