use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[derive(Clone, Copy, PartialOrd)]
pub struct FpWrapper(f64);

impl From<f32> for FpWrapper {
    fn from(value: f32) -> Self {
        FpWrapper(value as f64)
    }
}

impl From<f64> for FpWrapper {
    fn from(value: f64) -> Self {
        FpWrapper(value)
    }
}

impl PartialEq for FpWrapper {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_finite() && other.0.is_finite() {
            self.0 == other.0
        } else {
            self.0.classify() == other.0.classify()
        }
    }
}

// We understand why implementing Eq and Hash for floating point numbers is problematic in general,
// but for our purposes, it should be fine.
impl Eq for FpWrapper {}

impl Hash for FpWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ne_bytes().hash(state)
    }
}

impl fmt::Display for FpWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for FpWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Deref for FpWrapper {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DataHolder<T> {
    pub value: T,
    pub width: u8,
}

impl<T> DataHolder<T> {
    pub fn new(value: T, width: u8) -> Self {
        DataHolder { value, width }
    }
}

impl<T> Deref for DataHolder<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: PartialOrd> PartialOrd for DataHolder<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

macro_rules! impl_data_holder_for {
    ($orig:ty, $holder:ty, $width:expr) => {
        impl From<$orig> for DataHolder<$holder> {
            fn from(value: $orig) -> Self {
                DataHolder::new(value.into(), $width)
            }
        }
    };
}

impl_data_holder_for!(i8, i64, 8);
impl_data_holder_for!(i16, i64, 16);
impl_data_holder_for!(i32, i64, 32);
impl_data_holder_for!(i64, i64, 64);
impl_data_holder_for!(u8, u64, 8);
impl_data_holder_for!(u16, u64, 16);
impl_data_holder_for!(u32, u64, 32);
impl_data_holder_for!(u64, u64, 64);
impl_data_holder_for!(f32, FpWrapper, 32);
impl_data_holder_for!(f64, FpWrapper, 64);

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum Value {
    Unsupported,
    Signed(DataHolder<i64>),
    Unsigned(DataHolder<u64>),
    Floating(DataHolder<FpWrapper>),
    Boolean(bool),
}

impl Value {
    pub fn unsupported() -> Self {
        Value::Unsupported
    }

    pub fn signed<T: Into<DataHolder<i64>>>(value: T) -> Self {
        Value::Signed(value.into())
    }

    pub fn unsigned<T: Into<DataHolder<u64>>>(value: T) -> Self {
        Value::Unsigned(value.into())
    }

    pub fn floating<T: Into<DataHolder<FpWrapper>>>(value: T) -> Self {
        Value::Floating(value.into())
    }

    pub fn boolean(value: bool) -> Self {
        Value::Boolean(value)
    }

    pub fn get_type(&self) -> ValueType {
        match self {
            Value::Unsupported => ValueType::Unsupported,
            Value::Signed(value) => ValueType::Signed(value.width),
            Value::Unsigned(value) => ValueType::Unsigned(value.width),
            Value::Floating(value) => ValueType::Floating(value.width),
            Value::Boolean(_) => ValueType::Boolean,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Value::Unsupported => false,
            Value::Signed(value) => **value == 0,
            Value::Unsigned(value) => **value == 0,
            Value::Floating(value) => ***value == 0.0,
            Value::Boolean(value) => !value,
        }
    }

    pub fn is_unsupported(&self) -> bool {
        match self {
            Value::Unsupported => true,
            _ => false,
        }
    }

    pub fn is_exceptional_value(&self) -> bool {
        match self {
            Value::Floating(value) => !(***value).is_finite(),
            _ => false,
        }
    }

    pub fn as_signed(&self) -> Option<i64> {
        match self {
            Value::Signed(value) => Some(**value),
            _ => None,
        }
    }

    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            Value::Unsigned(value) => Some(**value),
            _ => None,
        }
    }

    pub fn as_floating(&self) -> Option<f64> {
        match self {
            Value::Floating(value) => Some(***value),
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
            Value::Signed(value) => write!(f, "{}", **value),
            Value::Unsigned(value) => write!(f, "{}", **value),
            Value::Floating(value) => write!(f, "{}", ***value),
            Value::Boolean(value) => write!(f, "{}", value),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Unsupported, Value::Unsupported) => Some(Ordering::Equal),
            (Value::Signed(lhs), Value::Signed(rhs)) => lhs.partial_cmp(rhs),
            (Value::Unsigned(lhs), Value::Unsigned(rhs)) => lhs.partial_cmp(rhs),
            (Value::Floating(lhs), Value::Floating(rhs)) => lhs.partial_cmp(rhs),
            (Value::Boolean(lhs), Value::Boolean(rhs)) => lhs.partial_cmp(rhs),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ValueType {
    Unsupported,
    Signed(u8),
    Unsigned(u8),
    Floating(u8),
    Boolean,
}

impl ValueType {
    pub fn is_signed(&self) -> bool {
        match self {
            ValueType::Signed(_) => true,
            _ => false,
        }
    }

    pub fn is_unsigned(&self) -> bool {
        match self {
            ValueType::Unsigned(_) => true,
            _ => false,
        }
    }

    pub fn is_floating(&self) -> bool {
        match self {
            ValueType::Floating(_) => true,
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        self.is_signed() || self.is_unsigned() || self.is_floating()
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            ValueType::Boolean => true,
            _ => false,
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Unsupported => write!(f, "?"),
            ValueType::Signed(width) => write!(f, "{}-bit signed integer", width),
            ValueType::Unsigned(width) => write!(f, "{}-bit unsigned integer", width),
            ValueType::Floating(32) => write!(f, "float"),
            ValueType::Floating(64) => write!(f, "double"),
            ValueType::Floating(width) => write!(f, "{}-bit floating point", width),
            ValueType::Boolean => write!(f, "boolean"),
        }
    }
}
