//! Data related to the instrumented program execution.

use super::types::{StmtId, TestName};
use super::values::ValueRef;
use crate::arena::S;

/// An item in the trace.
pub enum TraceItem {
    /// Statement execution.
    Statement(StmtId),
    /// Indication of test case beginning.
    Test(S<TestName>),
    /// Variable value.
    Value(ValueRef),
}

/// Runtime trace.
///
/// It is simply a long sequence of items.
pub struct Trace {
    pub trace: Vec<TraceItem>,
}

impl Trace {
    /// Initializes empty data.
    pub(crate) fn new() -> Self {
        Trace { trace: Vec::new() }
    }

    /// Filters the trace such that all the items belong to just the given test
    /// case.
    pub fn find_test(&self, test: &S<TestName>) -> TestTraceIter<'_> {
        // FIXME: Very inefficient. Probably use global Vars query as before.
        TestTraceIter {
            inner: self.trace.iter(),
            state: TestTraceIterState::Fresh,
            test: *test,
        }
    }
}

enum TestTraceIterState {
    Fresh,
    Within,
    Finished,
}

/// Iterator over the trace where all items belong to a single test case.
pub struct TestTraceIter<'a> {
    inner: std::slice::Iter<'a, TraceItem>,
    state: TestTraceIterState,
    test: S<TestName>,
}

impl<'a> Iterator for TestTraceIter<'a> {
    type Item = &'a TraceItem;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            TestTraceIterState::Fresh => {
                loop {
                    match self.inner.next() {
                        Some(TraceItem::Test(test)) if test == &self.test => {
                            self.state = TestTraceIterState::Within;
                            return self.inner.next();
                        }
                        None => {
                            self.state = TestTraceIterState::Finished;
                            return None;
                        }
                        _ => { /* Just keep searching */ }
                    }
                }
            }
            TestTraceIterState::Within => match self.inner.next() {
                Some(TraceItem::Test(_)) | None => {
                    self.state = TestTraceIterState::Finished;
                    None
                }
                next => next,
            },
            TestTraceIterState::Finished => None,
        }
    }
}
