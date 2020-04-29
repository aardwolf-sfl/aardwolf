use std::collections::HashMap;

use super::types::TestName;

#[derive(PartialEq, Eq)]
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

pub struct TestSuite {
    pub tests: HashMap<TestName, TestStatus>,
}

impl TestSuite {
    pub(crate) fn new() -> Self {
        TestSuite {
            tests: HashMap::new(),
        }
    }
}
