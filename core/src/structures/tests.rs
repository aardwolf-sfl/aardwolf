use std::collections::hash_map::{HashMap, Iter, Keys};
use std::mem;

use crate::api::Api;
use crate::raw::data::{Data, Statement, TestData, TestName, TestStatus, TraceItem};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Tests<'a> {
    raw: &'a TestData,
    traces: HashMap<&'a TestName, Vec<&'a Statement>>,
}

impl<'a> Tests<'a> {
    pub fn iter_names(&self) -> Keys<TestName, TestStatus> {
        self.raw.tests.keys()
    }

    pub fn iter_statuses(&self) -> Iter<TestName, TestStatus> {
        self.raw.tests.iter()
    }

    pub fn iter_stmts(&'a self, test: &'a TestName) -> Option<impl Iterator<Item = &'a Statement>> {
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

impl<'a> FromRawData<'a> for Tests<'a> {
    fn from_raw(data: &'a Data, api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
        let stmts = api.get_stmts();

        let mut traces = HashMap::with_capacity(data.test_data.tests.len());

        let mut test = None;
        let mut trace = Vec::new();

        for item in data.dynamic_data.trace.iter() {
            match item {
                TraceItem::Statement(id) => {
                    // Stmts are built from dynamic trace so a statement with this id certainly exists.
                    trace.push(stmts.get(id).unwrap());
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
