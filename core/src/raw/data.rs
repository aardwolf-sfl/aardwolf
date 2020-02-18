use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(PartialEq, Eq, Hash)]
pub enum Access {
    Scalar(u64),
    Structural(Box<Access>, Box<Access>),
    ArrayLike(Box<Access>, Vec<Access>),
}

impl Access {
    // TODO: Split this into two functions - one for uses, one for defs
    // (they differ in case of non-scalar accesses).
    pub fn get_scalars(&self) -> Vec<u64> {
        let mut result = Vec::new();
        self.get_scalars_rec(&mut result);
        result
    }

    fn get_scalars_rec(&self, result: &mut Vec<u64>) {
        match self {
            Access::Scalar(scalar) => result.push(*scalar),
            Access::Structural(obj, field) => {
                obj.get_scalars_rec(result);
                field.get_scalars_rec(result);
            }
            Access::ArrayLike(array, index) => {
                array.get_scalars_rec(result);
                for index_var in index {
                    index_var.get_scalars_rec(result);
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

pub enum TraceItem {
    Statement(u64),
    External(String),
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
                write!(f, "{:?}[{:?}", base, index[0])?;
                for item in index.iter().skip(1) {
                    write!(f, ", {:?}", item)?;
                }
                write!(f, "]")
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

impl fmt::Debug for TraceItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceItem::Statement(id) => write!(f, "statement: #{}", id),
            TraceItem::External(external) => write!(f, "external: \"{}\"", external),
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
