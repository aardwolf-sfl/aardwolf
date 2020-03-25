use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use crate::api::Api;

#[derive(PartialEq, Eq, Hash)]
pub enum Access {
    Scalar(u64),
    Structural(Box<Access>, Box<Access>),
    ArrayLike(Box<Access>, Vec<Access>),
}

impl Access {
    pub fn get_scalars_for_def(&self) -> Vec<u64> {
        let mut result = Vec::new();

        match self {
            Access::Scalar(scalar) => result.push(*scalar),
            access => access.get_scalars_for_def_rec(&mut result),
        }

        result
    }

    pub fn get_scalars_for_def_rec(&self, result: &mut Vec<u64>) {
        match self {
            Access::Scalar(scalar) => result.push(*scalar),
            Access::Structural(obj, field) => {
                obj.get_scalars_for_use_rec(result);

                match field.as_ref() {
                    // Fields of a structure are not considered to be defined (the containing structure is).
                    Access::Scalar(_) => {}
                    field => field.get_scalars_for_use_rec(result),
                }
            }
            Access::ArrayLike(array, _) => {
                array.get_scalars_for_use_rec(result);
                // Variables in the index to the array are not defined by the statement.
            }
        }
    }

    pub fn get_scalars_for_use(&self) -> Vec<u64> {
        let mut result = Vec::new();
        self.get_scalars_for_use_rec(&mut result);
        result
    }

    fn get_scalars_for_use_rec(&self, result: &mut Vec<u64>) {
        match self {
            Access::Scalar(scalar) => result.push(*scalar),
            Access::Structural(obj, field) => {
                obj.get_scalars_for_use_rec(result);
                field.get_scalars_for_use_rec(result);
            }
            Access::ArrayLike(array, index) => {
                array.get_scalars_for_use_rec(result);
                for index_var in index {
                    index_var.get_scalars_for_use_rec(result);
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Loc {
    pub file_id: u64,
    pub line_begin: u32,
    pub col_begin: u32,
    pub line_end: u32,
    pub col_end: u32,
}

impl Loc {
    pub fn merge(&self, other: &Self) -> Self {
        let line_begin = u32::min(self.line_begin, other.line_begin);
        let col_begin = u32::min(self.col_begin, other.col_begin);
        let line_end = u32::max(self.line_end, other.line_end);
        let col_end = u32::max(self.col_end, other.col_end);
        Loc {
            file_id: self.file_id,
            line_begin,
            col_begin,
            line_end,
            col_end,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        let file = self.file_id == other.file_id;

        let begin = if self.line_begin == other.line_begin {
            self.col_begin <= other.col_begin
        } else {
            self.line_begin <= other.line_begin
        };

        let end = if self.line_end == other.line_end {
            self.col_end >= other.col_end
        } else {
            self.line_end >= other.line_end
        };

        file && begin && end
    }

    pub const fn dummy() -> Self {
        Loc {
            file_id: 0,
            line_begin: 0,
            col_begin: 0,
            line_end: 0,
            col_end: 0,
        }
    }

    pub fn to_string<'data>(&self, api: &'data Api<'data>) -> String {
        let mut loc = String::new();
        loc.push_str(api.get_filepath(self.file_id).unwrap().to_str().unwrap());
        loc.push(':');

        if self.line_begin == self.line_end && self.col_begin == self.col_end {
            loc.push_str(&format!("{}:{}", self.line_begin, self.col_begin));
        } else {
            loc.push_str(&format!("{}-{}", self.line_begin, self.line_end));
        }

        loc
    }
}

pub type StmtId = (u64, u64);

pub struct Statement {
    pub id: StmtId,
    pub succ: Vec<StmtId>,
    pub defs: Vec<Access>,
    pub uses: Vec<Access>,
    pub loc: Loc,
    pub metadata: u8,
}

pub struct StaticData {
    pub functions: HashMap<String, HashMap<StmtId, Statement>>,
    pub files: HashMap<u64, String>,
}

impl Default for StaticData {
    fn default() -> Self {
        StaticData {
            functions: HashMap::new(),
            files: HashMap::new(),
        }
    }
}

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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum VariableData {
    Unsupported,
    Signed(DataHolder<i64>),
    Unsigned(DataHolder<u64>),
    Floating(DataHolder<FpWrapper>),
    Boolean(bool),
}

impl VariableData {
    pub fn unsupported() -> Self {
        VariableData::Unsupported
    }

    pub fn signed<T: Into<DataHolder<i64>>>(value: T) -> Self {
        VariableData::Signed(value.into())
    }

    pub fn unsigned<T: Into<DataHolder<u64>>>(value: T) -> Self {
        VariableData::Unsigned(value.into())
    }

    pub fn floating<T: Into<DataHolder<FpWrapper>>>(value: T) -> Self {
        VariableData::Floating(value.into())
    }

    pub fn boolean(value: bool) -> Self {
        VariableData::Boolean(value)
    }

    pub fn get_type(&self) -> VariableDataType {
        match self {
            VariableData::Unsupported => VariableDataType::Unsupported,
            VariableData::Signed(value) => VariableDataType::Signed(value.width),
            VariableData::Unsigned(value) => VariableDataType::Unsigned(value.width),
            VariableData::Floating(value) => VariableDataType::Floating(value.width),
            VariableData::Boolean(_) => VariableDataType::Boolean,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            VariableData::Unsupported => false,
            VariableData::Signed(value) => **value == 0,
            VariableData::Unsigned(value) => **value == 0,
            VariableData::Floating(value) => ***value == 0.0,
            VariableData::Boolean(value) => !value,
        }
    }

    pub fn is_unsupported(&self) -> bool {
        match self {
            VariableData::Unsupported => true,
            _ => false,
        }
    }

    pub fn is_exceptional_value(&self) -> bool {
        match self {
            VariableData::Floating(value) => !(***value).is_finite(),
            _ => false,
        }
    }

    pub fn as_signed(&self) -> Option<i64> {
        match self {
            VariableData::Signed(value) => Some(**value),
            _ => None,
        }
    }

    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            VariableData::Unsigned(value) => Some(**value),
            _ => None,
        }
    }

    pub fn as_floating(&self) -> Option<f64> {
        match self {
            VariableData::Floating(value) => Some(***value),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            VariableData::Boolean(value) => Some(*value),
            _ => None,
        }
    }
}

impl fmt::Display for VariableData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableData::Unsupported => write!(f, "?"),
            VariableData::Signed(value) => write!(f, "{}", **value),
            VariableData::Unsigned(value) => write!(f, "{}", **value),
            VariableData::Floating(value) => write!(f, "{}", ***value),
            VariableData::Boolean(value) => write!(f, "{}", value),
        }
    }
}

impl PartialOrd for VariableData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (VariableData::Unsupported, VariableData::Unsupported) => Some(Ordering::Equal),
            (VariableData::Signed(lhs), VariableData::Signed(rhs)) => lhs.partial_cmp(rhs),
            (VariableData::Unsigned(lhs), VariableData::Unsigned(rhs)) => lhs.partial_cmp(rhs),
            (VariableData::Floating(lhs), VariableData::Floating(rhs)) => lhs.partial_cmp(rhs),
            (VariableData::Boolean(lhs), VariableData::Boolean(rhs)) => lhs.partial_cmp(rhs),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VariableDataType {
    Unsupported,
    Signed(u8),
    Unsigned(u8),
    Floating(u8),
    Boolean,
}

impl VariableDataType {
    pub fn is_signed(&self) -> bool {
        match self {
            VariableDataType::Signed(_) => true,
            _ => false,
        }
    }

    pub fn is_unsigned(&self) -> bool {
        match self {
            VariableDataType::Unsigned(_) => true,
            _ => false,
        }
    }

    pub fn is_floating(&self) -> bool {
        match self {
            VariableDataType::Floating(_) => true,
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        self.is_signed() || self.is_unsigned() || self.is_floating()
    }

    pub fn is_boolean(&self) -> bool {
        match self {
            VariableDataType::Boolean => true,
            _ => false,
        }
    }
}

impl fmt::Display for VariableDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableDataType::Unsupported => write!(f, "?"),
            VariableDataType::Signed(width) => write!(f, "{}-bit signed integer", width),
            VariableDataType::Unsigned(width) => write!(f, "{}-bit unsigned integer", width),
            VariableDataType::Floating(32) => write!(f, "float"),
            VariableDataType::Floating(64) => write!(f, "double"),
            VariableDataType::Floating(width) => write!(f, "{}-bit floating point", width),
            VariableDataType::Boolean => write!(f, "boolean"),
        }
    }
}

pub enum TraceItem {
    Statement(StmtId),
    External(String),
    Data(VariableData),
}

pub struct DynamicData {
    pub trace: Vec<TraceItem>,
}

#[derive(PartialEq)]
pub enum TestStatus {
    Failed,
    Passed,
}

impl TestStatus {
    pub fn is_failed(&self) -> bool {
        self == &TestStatus::Failed
    }

    pub fn is_passed(&self) -> bool {
        self == &TestStatus::Passed
    }
}

pub type TestName = String;

pub struct TestData {
    pub tests: HashMap<TestName, TestStatus>,
}

pub struct Data {
    pub static_data: StaticData,
    pub dynamic_data: DynamicData,
    pub test_data: TestData,
}

impl Statement {
    pub const fn dummy(id: StmtId) -> Self {
        Statement {
            id,
            succ: Vec::new(),
            defs: Vec::new(),
            uses: Vec::new(),
            loc: Loc::dummy(),
            metadata: 0,
        }
    }

    pub fn is_arg(&self) -> bool {
        self.is_meta(0x61)
    }

    pub fn is_ret(&self) -> bool {
        self.is_meta(0x62)
    }

    pub fn is_call(&self) -> bool {
        self.is_meta(0x64)
    }

    pub fn is_predicate(&self) -> bool {
        self.succ.len() > 1
    }

    pub fn is_succ(&self, stmt: &Statement) -> bool {
        self.succ.iter().any(|succ| succ == &stmt.id)
    }

    pub fn has_meta(&self) -> bool {
        self.metadata != 0
    }

    fn is_meta(&self, identifier: u8) -> bool {
        let meta = 0x60;
        (self.metadata & !meta) == (identifier & !meta)
    }
}

impl fmt::Debug for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Access::Scalar(id) => write!(f, "%{}", id),
            Access::Structural(base, field) => write!(f, "{:?}.{:?}", base, field),
            Access::ArrayLike(base, index) => {
                if index.is_empty() {
                    write!(f, "{:?}[]", base)
                } else {
                    write!(f, "{:?}[{:?}", base, index[0])?;
                    for item in index.iter().skip(1) {
                        write!(f, ", {:?}", item)?;
                    }
                    write!(f, "]")
                }
            }
        }
    }
}

impl fmt::Debug for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "@{} {}:{}-{}:{}",
            self.file_id, self.line_begin, self.col_begin, self.line_end, self.col_end
        )
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}:{} -> ", self.id.0, self.id.1)?;

        let mut succ_iter = self.succ.iter();
        if let Some(succ) = succ_iter.next() {
            write!(f, "#{}:{}", succ.0, succ.1)?;

            while let Some(succ) = succ_iter.next() {
                write!(f, ", #{}:{}", succ.0, succ.1)?;
            }
        }

        write!(f, "  ::  defs: ")?;

        let mut defs_iter = self.defs.iter();
        if let Some(def_value) = defs_iter.next() {
            write!(f, "{:?}", def_value)?;

            while let Some(def_value) = defs_iter.next() {
                write!(f, ", {:?}", def_value)?;
            }
        }

        write!(f, " / uses: ")?;

        let mut uses_iter = self.uses.iter();
        if let Some(use_value) = uses_iter.next() {
            write!(f, "{:?}", use_value)?;

            while let Some(use_value) = uses_iter.next() {
                write!(f, ", {:?}", use_value)?;
            }
        }

        write!(f, " [{:?}]", self.loc)?;

        if self.has_meta() {
            write!(f, "  {{ ")?;

            if self.is_arg() {
                write!(f, "arg")?;
            }

            if self.is_ret() {
                if self.is_arg() {
                    write!(f, ", ")?;
                }

                write!(f, "ret")?;
            }

            write!(f, " }}")?;
        }

        Ok(())
    }
}

impl PartialEq for Statement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Statement {}

impl Hash for Statement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for Statement {
    fn partial_cmp(&self, other: &Statement) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Statement {
    fn cmp(&self, other: &Statement) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl fmt::Debug for StaticData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sorted_functions: Vec<_> = self.functions.iter().collect();
        // Sort by ids of whatever statement in function bodies in order to output log globally sorted by statement ids.
        // We assume functions to be non-empty and the sets of statement ids to be disjoint.
        sorted_functions.sort_unstable_by(|a, b| {
            a.1.iter()
                .next()
                .unwrap()
                .0
                .cmp(b.1.iter().next().unwrap().0)
        });

        for (function, statements) in sorted_functions {
            write!(f, "\nfunction: {}\n\n", function)?;

            let mut sorted_statements: Vec<_> = statements.iter().map(|(_, stmt)| stmt).collect();
            sorted_statements.sort_unstable_by(|a, b| a.id.cmp(&b.id));

            for statement in sorted_statements {
                write!(f, "{:?}\n", statement)?;
            }
        }

        write!(f, "\n")?;

        for (file_id, filepath) in self.files.iter() {
            write!(f, "@{} = {}\n", file_id, filepath)?;
        }

        Ok(())
    }
}

impl fmt::Debug for VariableData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableData::Unsupported => write!(f, "unsupported"),
            VariableData::Signed(value) => write!(f, "{}: i{}", **value, value.width),
            VariableData::Unsigned(value) => write!(f, "{}: u{}", **value, value.width),
            VariableData::Floating(value) => write!(f, "{}: f{}", **value, value.width),
            VariableData::Boolean(value) => write!(f, "{}: boolean", value),
        }
    }
}

impl fmt::Debug for TraceItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceItem::Statement(id) => write!(f, "statement: #{}:{}", id.0, id.1),
            TraceItem::External(external) => write!(f, "external: \"{}\"", external),
            TraceItem::Data(data) => write!(f, "data: {:?}", data),
        }
    }
}

impl fmt::Debug for DynamicData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in self.trace.iter() {
            write!(f, "{:?}\n", item)?;
        }

        Ok(())
    }
}

impl fmt::Debug for TestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "PASSED"),
            TestStatus::Failed => write!(f, "FAILED"),
        }
    }
}

impl fmt::Debug for TestData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (name, status) in self.tests.iter() {
            write!(f, "\"{}\": {:?}\n", name, status)?;
        }

        Ok(())
    }
}
