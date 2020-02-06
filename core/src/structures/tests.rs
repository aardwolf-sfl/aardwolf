use std::collections::hash_map::{Iter, Keys};

use crate::api::Api;
use crate::raw::data::{Data, TestData, TestName, TestStatus};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Tests<'a>(&'a TestData);

impl<'a> Tests<'a> {
    pub fn iter_names(&self) -> Keys<TestName, TestStatus> {
        self.0.tests.keys()
    }

    pub fn iter(&self) -> Iter<TestName, TestStatus> {
        self.0.tests.iter()
    }

    pub fn is_passed(&self, test: &TestName) -> bool {
        if let Some(status) = self.0.tests.get(test) {
            *status == TestStatus::Passed
        } else {
            false
        }
    }
}

impl<'a> FromRawData<'a> for Tests<'a> {
    fn from_raw(data: &'a Data, _api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
        Ok(Tests(&data.test_data))
    }
}
