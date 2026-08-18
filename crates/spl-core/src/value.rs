//! Core A value domain.
//!
//! Core A values are nominal: a `U8` is not an `I32` that happens to be small,
//! and there is no generic integer that later gets reinterpreted by scattered
//! casts. Every operation in this crate matches on the pair of nominal types and
//! either has a defined meaning for it or rejects it.

use core::fmt;

/// The Core A types. Nothing else exists in Stage 1.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Type {
    Bool,
    U8,
    U32,
    I32,
}

impl Type {
    /// Whether arithmetic (wrapping or checked) is defined for this type.
    pub fn is_arithmetic(self) -> bool {
        matches!(self, Type::U8 | Type::U32 | Type::I32)
    }

    /// Whether the type is ordered, i.e. `<`, `<=`, `>`, `>=` are defined.
    ///
    /// `Bool` supports only equality in Core A. Ordering booleans would import
    /// a convention (false < true) that the semantic contract has no reason to
    /// commit to at this stage.
    pub fn is_ordered(self) -> bool {
        self.is_arithmetic()
    }

    pub fn name(self) -> &'static str {
        match self {
            Type::Bool => "Bool",
            Type::U8 => "u8",
            Type::U32 => "u32",
            Type::I32 => "i32",
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A Core A value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Value {
    Bool(bool),
    U8(u8),
    U32(u32),
    I32(i32),
}

impl Value {
    pub fn ty(self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,
            Value::U8(_) => Type::U8,
            Value::U32(_) => Type::U32,
            Value::I32(_) => Type::I32,
        }
    }

    /// The value as an exact mathematical integer, for the integer types only.
    ///
    /// This is a *view*, not a semantic operation: no Core A operation is
    /// defined through it. It exists so that encoders and diagnostics do not
    /// each reimplement the match.
    pub fn as_integer(self) -> Option<i128> {
        match self {
            Value::Bool(_) => None,
            Value::U8(v) => Some(i128::from(v)),
            Value::U32(v) => Some(i128::from(v)),
            Value::I32(v) => Some(i128::from(v)),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(v) => write!(f, "{v}:Bool"),
            Value::U8(v) => write!(f, "{v}:u8"),
            Value::U32(v) => write!(f, "{v}:u32"),
            Value::I32(v) => write!(f, "{v}:i32"),
        }
    }
}
