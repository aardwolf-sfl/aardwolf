use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

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

#[derive(Clone, Copy)]
pub struct Loc {
    pub file_id: u32,
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

    pub const fn dummy() -> Self {
        Loc {
            file_id: 0,
            line_begin: 0,
            col_begin: 0,
            line_end: 0,
            col_end: 0,
        }
    }
}

pub struct Statement {
    pub id: u64,
    pub succ: Vec<u64>,
    pub defs: Vec<Access>,
    pub uses: Vec<Access>,
    pub loc: Loc,
    pub metadata: u8,
}

pub struct StaticData {
    pub functions: HashMap<String, HashMap<u64, Statement>>,
    pub files: HashMap<u32, String>,
}

// We understand why implementing Eq and Hash for floating point numbers is problematic in general,
// but for our purposes, it should be fine.
macro_rules! impl_fp_wrapper {
    ($name:ident, $typ:ty) => {
        #[derive(Clone, Copy)]
        pub struct $name($typ);

        impl $name {
            pub fn new(value: $typ) -> Self {
                $name(value)
            }
        }

        impl From<$typ> for $name {
            fn from(value: $typ) -> Self {
                $name::new(value)
            }
        }

        impl Into<f64> for $name {
            fn into(self) -> f64 {
                self.0 as f64
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                if self.0.is_finite() && other.0.is_finite() {
                    self.0 == other.0
                } else {
                    self.0.classify() == other.0.classify()
                }
            }
        }

        impl Eq for $name {}

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.to_ne_bytes().hash(state)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:?}", self.0)
            }
        }

        impl Deref for $name {
            type Target = $typ;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

impl_fp_wrapper!(F32, f32);
impl_fp_wrapper!(F64, f64);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum VariableData {
    Unsupported,
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(F32),
    F64(F64),
}

impl VariableData {
    pub fn get_type(&self) -> VariableDataType {
        match self {
            VariableData::Unsupported => VariableDataType::Unsupported,
            VariableData::I8(_) => VariableDataType::I8,
            VariableData::I16(_) => VariableDataType::I16,
            VariableData::I32(_) => VariableDataType::I32,
            VariableData::I64(_) => VariableDataType::I64,
            VariableData::U8(_) => VariableDataType::U8,
            VariableData::U16(_) => VariableDataType::U16,
            VariableData::U32(_) => VariableDataType::U32,
            VariableData::U64(_) => VariableDataType::U64,
            VariableData::F32(_) => VariableDataType::F32,
            VariableData::F64(_) => VariableDataType::F64,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            VariableData::Unsupported => false,
            VariableData::I8(x) => *x == 0,
            VariableData::I16(x) => *x == 0,
            VariableData::I32(x) => *x == 0,
            VariableData::I64(x) => *x == 0,
            VariableData::U8(x) => *x == 0,
            VariableData::U16(x) => *x == 0,
            VariableData::U32(x) => *x == 0,
            VariableData::U64(x) => *x == 0,
            VariableData::F32(x) => **x == 0.0,
            VariableData::F64(x) => **x == 0.0,
        }
    }

    pub fn as_signed(&self) -> Option<i64> {
        match self {
            VariableData::I8(x) => Some(*x as i64),
            VariableData::I16(x) => Some(*x as i64),
            VariableData::I32(x) => Some(*x as i64),
            VariableData::I64(x) => Some(*x as i64),
            _ => None,
        }
    }

    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            VariableData::U8(x) => Some(*x as u64),
            VariableData::U16(x) => Some(*x as u64),
            VariableData::U32(x) => Some(*x as u64),
            VariableData::U64(x) => Some(*x as u64),
            _ => None,
        }
    }

    pub fn as_floating(&self) -> Option<f64> {
        match self {
            VariableData::F32(x) => Some(**x as f64),
            VariableData::F64(x) => Some(**x as f64),
            _ => None,
        }
    }
}

impl fmt::Display for VariableData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableData::Unsupported => write!(f, "?"),
            VariableData::I8(x) => write!(f, "{}", x),
            VariableData::I16(x) => write!(f, "{}", x),
            VariableData::I32(x) => write!(f, "{}", x),
            VariableData::I64(x) => write!(f, "{}", x),
            VariableData::U8(x) => write!(f, "{}", x),
            VariableData::U16(x) => write!(f, "{}", x),
            VariableData::U32(x) => write!(f, "{}", x),
            VariableData::U64(x) => write!(f, "{}", x),
            VariableData::F32(x) => write!(f, "{}", x),
            VariableData::F64(x) => write!(f, "{}", x),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum VariableDataType {
    Unsupported,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
}

impl VariableDataType {
    pub fn is_signed(&self) -> bool {
        match self {
            VariableDataType::I8
            | VariableDataType::I16
            | VariableDataType::I32
            | VariableDataType::I64 => true,
            _ => false,
        }
    }

    pub fn is_unsigned(&self) -> bool {
        match self {
            VariableDataType::U8
            | VariableDataType::U16
            | VariableDataType::U32
            | VariableDataType::U64 => true,
            _ => false,
        }
    }

    pub fn is_floating(&self) -> bool {
        match self {
            VariableDataType::F32 | VariableDataType::F64 => true,
            _ => false,
        }
    }

    pub fn is_numeric(&self) -> bool {
        self.is_signed() || self.is_unsigned() || self.is_floating()
    }
}

impl fmt::Display for VariableDataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableDataType::Unsupported => write!(f, "?"),
            VariableDataType::I8 => write!(f, "8-bit signed integer"),
            VariableDataType::I16 => write!(f, "16-bit signed integer"),
            VariableDataType::I32 => write!(f, "32-bit signed integer"),
            VariableDataType::I64 => write!(f, "64-bit signed integer"),
            VariableDataType::U8 => write!(f, "8-bit unsigned integer"),
            VariableDataType::U16 => write!(f, "16-bit unsigned integer"),
            VariableDataType::U32 => write!(f, "32-bit unsigned integer"),
            VariableDataType::U64 => write!(f, "64-bit unsigned integer"),
            VariableDataType::F32 => write!(f, "float"),
            VariableDataType::F64 => write!(f, "double"),
        }
    }
}

pub enum TraceItem {
    Statement(u64),
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
    pub const fn dummy(id: u64) -> Self {
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
        write!(f, "#{} -> ", self.id)?;

        let mut succ_iter = self.succ.iter();
        if let Some(succ) = succ_iter.next() {
            write!(f, "#{}", succ)?;

            while let Some(succ) = succ_iter.next() {
                write!(f, ", #{}", succ)?;
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
            VariableData::I8(value) => write!(f, "{}: i8", value),
            VariableData::I16(value) => write!(f, "{}: i16", value),
            VariableData::I32(value) => write!(f, "{}: i32", value),
            VariableData::I64(value) => write!(f, "{}: i64", value),
            VariableData::U8(value) => write!(f, "{}: u8", value),
            VariableData::U16(value) => write!(f, "{}: u16", value),
            VariableData::U32(value) => write!(f, "{}: u32", value),
            VariableData::U64(value) => write!(f, "{}: u64", value),
            VariableData::F32(value) => write!(f, "{}: f32", value),
            VariableData::F64(value) => write!(f, "{}: f64", value),
        }
    }
}

impl fmt::Debug for TraceItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceItem::Statement(id) => write!(f, "statement: #{}", id),
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
