//! Data related to variable trace.

use std::convert::TryInto;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Reference type for [`ValueArena`] container.
///
/// [`ValueArena`]: struct.ValueArena.html
#[derive(Clone, Copy, Debug)]
pub struct ValueRef {
    index: u32,
}

/// An arena-like collection for variable values.
///
/// The data are stored as raw bytes in a compressed form. Each item starts with
/// a token which determines the type of values that follows it. The actual
/// values are stored in a compressed forms if the value allows it (e.g., even
/// if the variable has type `i32`, if its value is `0`, it is encoded as one
/// byte).
pub struct ValueArena {
    storage: Vec<u8>,
}

impl ValueArena {
    /// Initializes empty arena.
    pub(crate) const fn empty() -> Self {
        ValueArena {
            storage: Vec::new(),
        }
    }

    /// Initializes empty arena with given capacity.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        ValueArena {
            storage: Vec::with_capacity(capacity),
        }
    }

    /// Allocates (i.e., stores into internal storage) given value of given data
    /// type. It returns the reference which can be used to obtain the allocated
    /// data back.
    pub(crate) fn alloc(&mut self, value: Value, value_type: ValueType) -> ValueRef {
        assert!(
            self.storage.len() <= u32::MAX as usize,
            "maximum number of values exceeded"
        );
        let ptr = ValueRef {
            index: self.storage.len() as u32,
        };

        macro_rules! compress_numeric {
            ($value:expr, $typ:ty, $type_const:expr) => {{
                self.storage.push($type_const);
                self.storage
                    .extend_from_slice(&($value as $typ).to_ne_bytes());
            }};
            ($value:expr, $typ:ty, $type_const:expr, $limit_half:ty) => {
                if ($value as $typ) <= (<$limit_half>::MAX as $typ) {
                    compress_numeric!(
                        $value as $limit_half,
                        $limit_half,
                        $type_const | consts::COMPRESS_HALF
                    )
                } else {
                    compress_numeric!($value, $typ, $type_const)
                }
            };
            ($value:expr, $typ:ty, $type_const:expr, $limit_half:ty, $limit_quarter:ty) => {
                if ($value as $typ) <= (<$limit_quarter>::MAX as $typ) {
                    compress_numeric!(
                        $value as $limit_quarter,
                        $limit_quarter,
                        $type_const | consts::COMPRESS_QUARTER
                    )
                } else {
                    compress_numeric!($value, $typ, $type_const, $limit_half)
                }
            };
        }

        match value_type {
            ValueType::Unsupported => self.storage.push(consts::TYPE_UNSUPPORTED),
            ValueType::U8 => compress_numeric!(value.as_unsigned().unwrap(), u8, consts::TYPE_U8),
            ValueType::U16 => {
                compress_numeric!(value.as_unsigned().unwrap(), u16, consts::TYPE_U16, u8)
            }
            ValueType::U32 => {
                compress_numeric!(value.as_unsigned().unwrap(), u32, consts::TYPE_U32, u16, u8)
            }
            ValueType::U64 => compress_numeric!(
                value.as_unsigned().unwrap(),
                u64,
                consts::TYPE_U64,
                u32,
                u16
            ),
            ValueType::I8 => compress_numeric!(value.as_signed().unwrap(), i8, consts::TYPE_I8),
            ValueType::I16 => {
                compress_numeric!(value.as_signed().unwrap(), i16, consts::TYPE_I16, i8)
            }
            ValueType::I32 => {
                compress_numeric!(value.as_signed().unwrap(), i32, consts::TYPE_I32, u16, i8)
            }
            ValueType::I64 => {
                compress_numeric!(value.as_signed().unwrap(), i64, consts::TYPE_I64, u32, u16)
            }
            ValueType::F32 => {
                compress_numeric!(value.as_floating().unwrap(), f32, consts::TYPE_F32)
            }
            ValueType::F64 => {
                compress_numeric!(value.as_floating().unwrap(), f64, consts::TYPE_F64, f32)
            }
            ValueType::Boolean => self
                .storage
                .extend_from_slice(&[consts::TYPE_BOOL, value.as_boolean().unwrap() as u8]),
        };

        ptr
    }

    /// Returns data type of given reference. This is very cheap operation since
    /// the data type is determined by the token where the reference directly
    /// points to. If you need just the type and not the value, prefer this
    /// method.
    pub fn value_type(&self, ptr: &ValueRef) -> ValueType {
        let byte = self.storage[ptr.index as usize];
        match byte & consts::MASK_TYPE {
            consts::TYPE_UNSUPPORTED => ValueType::Unsupported,
            consts::TYPE_U8 => ValueType::U8,
            consts::TYPE_U16 => ValueType::U16,
            consts::TYPE_U32 => ValueType::U32,
            consts::TYPE_U64 => ValueType::U64,
            consts::TYPE_I8 => ValueType::I8,
            consts::TYPE_I16 => ValueType::I16,
            consts::TYPE_I32 => ValueType::I32,
            consts::TYPE_I64 => ValueType::I64,
            consts::TYPE_F32 => ValueType::F32,
            consts::TYPE_F64 => ValueType::F64,
            consts::TYPE_BOOL => ValueType::Boolean,
            _ => panic!("Invalid value allocation."),
        }
    }

    /// Returns value and the actual data type of given reference. This
    /// generally involves decompression of the data. If you need just the type,
    /// prefer [`value_type`] method.
    ///
    /// [`value_type`]: struct.ValueArena.html#method.value_type
    pub fn value(&self, ptr: &ValueRef) -> (Value, ValueType) {
        let value_type = self.value_type(ptr);

        let bytes = &self.storage[(ptr.index as usize)..];
        let control_byte = bytes[0];
        let bytes = &bytes[1..];

        macro_rules! decompress_numeric {
            ($typ:ty) => {
                <$typ>::from_ne_bytes(bytes[0..std::mem::size_of::<$typ>()].try_into().unwrap())
            };
            ($typ:ty, $type_half:ty) => {
                if control_byte & consts::MASK_COMPRESS == consts::COMPRESS_HALF {
                    <$type_half>::from_ne_bytes(
                        bytes[0..std::mem::size_of::<$type_half>()]
                            .try_into()
                            .unwrap(),
                    ) as $typ
                } else {
                    decompress_numeric!($typ)
                }
            };
            ($typ:ty, $type_half:ty, $type_quarter:ty) => {
                if control_byte & consts::MASK_COMPRESS == consts::COMPRESS_QUARTER {
                    <$type_quarter>::from_ne_bytes(
                        bytes[0..std::mem::size_of::<$type_quarter>()]
                            .try_into()
                            .unwrap(),
                    ) as $typ
                } else {
                    decompress_numeric!($typ, $type_half)
                }
            };
        }

        let value = match value_type {
            ValueType::Unsupported => Value::Unsupported,
            ValueType::U8 => Value::Unsigned(bytes[0] as u64),
            ValueType::U16 => Value::Unsigned(decompress_numeric!(u16, u8) as u64),
            ValueType::U32 => Value::Unsigned(decompress_numeric!(u32, u16, u8) as u64),
            ValueType::U64 => Value::Unsigned(decompress_numeric!(u64, u32, u16)),
            ValueType::I8 => Value::Signed(bytes[0] as i64),
            ValueType::I16 => Value::Signed(decompress_numeric!(i16, i8) as i64),
            ValueType::I32 => Value::Signed(decompress_numeric!(i32, i16, i8) as i64),
            ValueType::I64 => Value::Signed(decompress_numeric!(i64, i32, i16)),
            ValueType::F32 => Value::Floating(decompress_numeric!(f32) as f64),
            ValueType::F64 => Value::Floating(decompress_numeric!(f64, f32)),
            ValueType::Boolean => Value::Boolean(bytes[0] > 0),
        };

        (value, value_type)
    }
}

/// Data type of a value.
///
/// This is the enumeration of all types which are supported by Aardwolf at the
/// moment. A dummy value `Unsupported` then represents all the types that are
/// not supported.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum ValueType {
    Unsupported,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Boolean,
}

/// Actual value of a variable.
///
/// The value is stored in the biggest representation of the data type. For
/// example, even if a variable is of type `i8`, it is given as `i64`. This
/// simplifies the usage as one does not need to do many type conversions on
/// their own.
#[derive(Clone, PartialOrd, Debug)]
pub enum Value {
    Unsupported,
    Unsigned(u64),
    Signed(i64),
    Floating(f64),
    Boolean(bool),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Unsupported, Value::Unsupported) => true,
            (Value::Unsigned(lhs), Value::Unsigned(rhs)) => lhs == rhs,
            (Value::Signed(lhs), Value::Signed(rhs)) => lhs == rhs,
            (Value::Floating(lhs), Value::Floating(rhs)) => lhs.to_ne_bytes() == rhs.to_ne_bytes(),
            (Value::Boolean(lhs), Value::Boolean(rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);

        match self {
            Value::Unsupported => {}
            Value::Unsigned(value) => value.hash(state),
            Value::Signed(value) => value.hash(state),
            Value::Floating(value) => value.to_ne_bytes().hash(state),
            Value::Boolean(value) => value.hash(state),
        }
    }
}

impl Value {
    pub fn is_supported(&self) -> bool {
        self != &Value::Unsupported
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Value::Unsupported => false,
            Value::Signed(value) => value == &0,
            Value::Unsigned(value) => value == &0,
            Value::Floating(value) => value == &0.0,
            Value::Boolean(value) => !value,
        }
    }

    /// Determines whether the value is exceptional.
    ///
    /// Currently, these values are considered exceptional:
    ///
    /// * When floating point number is not finite (i.e., is NaN or infinity).
    pub fn is_exceptional(&self) -> bool {
        match self {
            Value::Floating(value) => !value.is_finite(),
            _ => false,
        }
    }

    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            Value::Unsigned(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_signed(&self) -> Option<i64> {
        match self {
            Value::Signed(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_floating(&self) -> Option<f64> {
        match self {
            Value::Floating(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Boolean(value) => Some(*value),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Unsupported => write!(f, "?"),
            Value::Signed(value) => write!(f, "{}", value),
            Value::Unsigned(value) => write!(f, "{}", value),
            Value::Floating(value) => write!(f, "{}", value),
            Value::Boolean(value) => write!(f, "{}", value),
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Unsupported => write!(f, "?"),
            ValueType::U8 => write!(f, "byte"),
            ValueType::U16 => write!(f, "unsigned short"),
            ValueType::U32 => write!(f, "unsigned int"),
            ValueType::U64 => write!(f, "unsigned long"),
            ValueType::I8 => write!(f, "signed byte"),
            ValueType::I16 => write!(f, "short"),
            ValueType::I32 => write!(f, "int"),
            ValueType::I64 => write!(f, "long"),
            ValueType::F32 => write!(f, "float"),
            ValueType::F64 => write!(f, "double"),
            ValueType::Boolean => write!(f, "bool"),
        }
    }
}

/// Helper trait for conversions of real data types into Aardwolf's
/// representation.
pub trait IntoValue {
    fn into_value(self) -> (Value, ValueType);
}

macro_rules! impl_into_value {
    ($typ:ty, $target_type:ty, $variant:path, $value_type:expr) => {
        impl IntoValue for $typ {
            fn into_value(self) -> (Value, ValueType) {
                ($variant(self as $target_type), $value_type)
            }
        }
    };
}

impl_into_value!(u8, u64, Value::Unsigned, ValueType::U8);
impl_into_value!(u16, u64, Value::Unsigned, ValueType::U16);
impl_into_value!(u32, u64, Value::Unsigned, ValueType::U32);
impl_into_value!(u64, u64, Value::Unsigned, ValueType::U64);
impl_into_value!(i8, i64, Value::Signed, ValueType::I8);
impl_into_value!(i16, i64, Value::Signed, ValueType::I16);
impl_into_value!(i32, i64, Value::Signed, ValueType::I32);
impl_into_value!(i64, i64, Value::Signed, ValueType::I64);
impl_into_value!(f32, f64, Value::Floating, ValueType::F32);
impl_into_value!(f64, f64, Value::Floating, ValueType::F64);
impl_into_value!(bool, bool, Value::Boolean, ValueType::Boolean);

impl_arena_type!(ValueRef, ValueArena);

impl ValueRef {
    /// Gets the value of the reference from the arena.
    pub fn value(&self) -> Value {
        Self::arena().value(self).0
    }

    /// Gets the value type of the reference from the arena.
    pub fn value_type(&self) -> ValueType {
        Self::arena().value_type(self)
    }

    /// Gets both the value and its type of the reference from the arena.
    pub fn value_and_type(&self) -> (Value, ValueType) {
        Self::arena().value(self)
    }
}

mod consts {
    // First 6 bytes determine the data type. Last 2 bytes determine the
    // compression ratio.

    const SPLIT: u8 = 2;

    pub const TYPE_UNSUPPORTED: u8 = 0b000001 << SPLIT;
    pub const TYPE_U8: u8 = 0b000010 << SPLIT;
    pub const TYPE_U16: u8 = 0b000011 << SPLIT;
    pub const TYPE_U32: u8 = 0b000100 << SPLIT;
    pub const TYPE_U64: u8 = 0b000101 << SPLIT;
    pub const TYPE_I8: u8 = 0b000110 << SPLIT;
    pub const TYPE_I16: u8 = 0b000111 << SPLIT;
    pub const TYPE_I32: u8 = 0b001000 << SPLIT;
    pub const TYPE_I64: u8 = 0b001001 << SPLIT;
    pub const TYPE_F32: u8 = 0b001010 << SPLIT;
    pub const TYPE_F64: u8 = 0b001011 << SPLIT;
    pub const TYPE_BOOL: u8 = 0b001100 << SPLIT;

    // Default is NONE.
    #[allow(dead_code)]
    pub const COMPRESS_NONE: u8 = 0b00;
    pub const COMPRESS_HALF: u8 = 0b01;
    pub const COMPRESS_QUARTER: u8 = 0b10;

    pub const MASK_TYPE: u8 = 0xff << SPLIT;
    pub const MASK_COMPRESS: u8 = 0xff >> (8 - SPLIT);
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn arena_compress_decompress() {
        let mut arena = ValueArena::empty();
        let orig = Value::Signed(3);
        let orig_type = ValueType::I64;

        let ptr = arena.alloc(orig.clone(), orig_type);
        assert_eq!(arena.storage.len(), 3);
        let (parsed, parsed_type) = arena.value(&ptr);

        assert_eq!(orig, parsed);
        assert_eq!(orig_type, parsed_type);

        let orig = Value::Signed(70000);
        let orig_type = ValueType::I32;

        let ptr = arena.alloc(orig.clone(), orig_type);
        assert_eq!(arena.storage.len(), 8);
        let (parsed, parsed_type) = arena.value(&ptr);

        assert_eq!(orig, parsed);
        assert_eq!(orig_type, parsed_type);
    }
}
