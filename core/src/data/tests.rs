//! Data related to test cases.

use std::collections::HashMap;

use super::types::TestName;
use crate::arena::S;

/// Test status, i.e., if it is passing or failing.
#[derive(Clone, Copy, PartialEq, Eq)]
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

/// Data obtained from parsing the test results.
///
/// It is just a mapping from a test case name to its status.
pub struct TestSuite {
    pub tests: HashMap<S<TestName>, TestStatus>,
}

impl TestSuite {
    /// Initializes empty data.
    pub(crate) fn new() -> Self {
        TestSuite {
            tests: HashMap::new(),
        }
    }
}
