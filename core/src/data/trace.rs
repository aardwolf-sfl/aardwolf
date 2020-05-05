use super::types::{StmtId, TestName};
use super::values::Value;
use crate::arena::{P, S};

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

    pub fn find_test(&self, test: &S<TestName>) -> TestTraceIter<'_> {
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
