use super::types::{StmtId, TestName};
use super::values::Value;

pub enum TraceItem {
    Statement(StmtId),
    Test(TestName),
    Value(Value),
}

pub struct Trace {
    pub trace: Vec<TraceItem>,
}

impl Trace {
    pub(crate) fn new() -> Self {
        Trace { trace: Vec::new() }
    }
}
