use std::collections::hash_map::{HashMap, Iter, Keys};
use std::mem;

use crate::api::Api;
use crate::raw::data::{Data, Statement, TestData, TestName, TestStatus, TraceItem};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Tests<'data> {
    raw: &'data TestData,
    traces: HashMap<&'data TestName, Vec<&'data Statement>>,
}

impl<'data> Tests<'data> {
    pub fn iter_names(&self) -> Keys<TestName, TestStatus> {
        self.raw.tests.keys()
    }

    pub fn iter_statuses(&self) -> Iter<TestName, TestStatus> {
        self.raw.tests.iter()
    }

    pub fn iter_stmts(&'data self, test: &'data TestName) -> Option<impl Iterator<Item = &'data Statement>> {
        self.traces.get(test).map(|stmts| stmts.iter().copied())
    }

    pub fn is_passed(&self, test: &TestName) -> bool {
        if let Some(status) = self.raw.tests.get(test) {
            *status == TestStatus::Passed
        } else {
            false
        }
    }

    pub fn iter_passed(&self) -> impl Iterator<Item = &TestName> {
        self.iter_statuses()
            .filter(|(_, status)| **status == TestStatus::Passed)
            .map(|(name, _)| name)
    }

    pub fn iter_failed(&self) -> impl Iterator<Item = &TestName> {
        self.iter_statuses()
            .filter(|(_, status)| **status == TestStatus::Failed)
            .map(|(name, _)| name)
    }

    pub fn get_failed(&self) -> &TestName {
        // Aardwolf performs validation whether there is at least one failed test.
        // We can therefore unwrap the first value of the iterator.
        self.iter_failed().next().unwrap()
    }
}

impl<'data> FromRawData<'data> for Tests<'data> {
    fn from_raw(data: &'data Data, api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let stmts = api.get_stmts();

        let mut traces = HashMap::with_capacity(data.test_data.tests.len());

        let mut test = None;
        let mut trace = Vec::new();

        for item in data.dynamic_data.trace.iter() {
            match item {
                TraceItem::Statement(id) => {
                    // Even though stmts are built from dynamic trace as well,
                    // traced statements without accompanied "external" element
                    // are discarded from it.
                    if let Some(id) = stmts.get(id) {
                        trace.push(id);
                    }
                }
                TraceItem::External(new_test) => {
                    if let Some(test) = test {
                        // Insert the trace and clear reset the trace variable in one step.
                        traces.insert(test, mem::take(&mut trace));
                    } else {
                        // Clear the trace when when it is not empty
                        // as we don't have a test to associate the statements with anyway.
                        traces.clear();
                    }

                    test = Some(new_test);
                }
                TraceItem::Data(_) => {} // Ignore
            }
        }

        // Insert the statements that remain.
        if let Some(test) = test {
            traces.insert(test, mem::take(&mut trace));
        }

        Ok(Tests {
            raw: &data.test_data,
            traces,
        })
    }
}
