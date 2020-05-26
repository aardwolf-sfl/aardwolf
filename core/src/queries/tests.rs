//! Several utilities for test suite.

use std::collections::hash_map::{HashMap, Iter, Keys};
use std::mem;

use super::stmts::Stmts;
use super::Query;
use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{
    statement::Statement, tests::TestStatus, trace::TraceItem, types::TestName, RawData,
};

pub struct Tests {
    tests: HashMap<S<TestName>, TestStatus>,
    traces: HashMap<S<TestName>, Vec<P<Statement>>>,
}

impl Tests {
    /// Iterates over all test names in the test suite.
    pub fn iter_names(&self) -> Keys<S<TestName>, TestStatus> {
        self.tests.keys()
    }

    /// Iterates over all test result statuses in the test suite.
    pub fn iter_statuses(&self) -> Iter<S<TestName>, TestStatus> {
        self.tests.iter()
    }

    /// Iterates over all statements which were executed in given test case.
    pub fn iter_stmts(&self, test: &S<TestName>) -> Option<impl Iterator<Item = &P<Statement>>> {
        self.traces.get(test).map(|stmts| stmts.iter())
    }

    /// Checks if given test case is passing.
    pub fn is_passed(&self, test: &S<TestName>) -> bool {
        if let Some(status) = self.tests.get(test) {
            *status == TestStatus::Passed
        } else {
            false
        }
    }

    /// Iterates over all passing test cases.
    pub fn iter_passed(&self) -> impl Iterator<Item = &S<TestName>> {
        self.iter_statuses()
            .filter(|(_, status)| **status == TestStatus::Passed)
            .map(|(name, _)| name)
    }

    /// Iterates over all failing test cases.
    pub fn iter_failed(&self) -> impl Iterator<Item = &S<TestName>> {
        self.iter_statuses()
            .filter(|(_, status)| **status == TestStatus::Failed)
            .map(|(name, _)| name)
    }

    /// Gets the most relevant failed test case. This should be used when a
    /// localization techniques is designed to work on a single failing
    /// execution.
    pub fn get_failed(&self) -> &S<TestName> {
        // Aardwolf performs validation whether there is at least one failed test.
        // We can therefore unwrap the first value of the iterator.
        self.iter_failed().next().unwrap()
    }
}

impl Query for Tests {
    type Error = ();
    type Args = ();

    fn init(data: &RawData, _args: &Self::Args, api: &Api) -> Result<Self, Self::Error> {
        let stmts = api.query::<Stmts>()?;

        let mut traces = HashMap::with_capacity(data.test_suite.tests.len());

        let mut test = None;
        let mut trace = Vec::new();

        for item in data.trace.trace.iter() {
            match item {
                TraceItem::Statement(id) => {
                    // Even though stmts are built from dynamic trace as well,
                    // traced statements without accompanied "external" element
                    // are discarded from it.
                    if let Some(stmt) = stmts.get(&id) {
                        trace.push(stmt.clone());
                    }
                }
                TraceItem::Test(new_test) => {
                    if let Some(test) = test {
                        // Insert the trace and clear reset the trace variable in one step.
                        traces.insert(test, mem::take(&mut trace));
                    } else {
                        // Clear the trace when when it is not empty
                        // as we don't have a test to associate the statements with anyway.
                        traces.clear();
                    }

                    test = Some(new_test.clone());
                }
                TraceItem::Value(_) => {} // Ignore
            }
        }

        // Insert the statements that remain.
        if let Some(test) = test {
            traces.insert(test, mem::take(&mut trace));
        }

        Ok(Tests {
            tests: data.test_suite.tests.clone(),
            traces,
        })
    }
}
