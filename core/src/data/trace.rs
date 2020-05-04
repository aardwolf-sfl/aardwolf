use super::types::{StmtId, TestName};
use super::values::Value;
use crate::arena::{S, P};

pub enum TraceItem {
    Statement(StmtId),
    Test(S<TestName>),
    Value(P<Value>),
}

pub struct Trace {
    pub trace: Vec<TraceItem>,
}

impl Trace {
    pub(crate) fn new() -> Self {
        Trace { trace: Vec::new() }
    }
}
