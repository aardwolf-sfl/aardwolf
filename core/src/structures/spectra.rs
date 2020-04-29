use std::collections::{HashMap, HashSet};

use crate::api::Api;
use crate::data::{
    statement::Statement,
    trace::TraceItem,
    types::{StmtId, TestName},
    RawData,
};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Spectra {
    spectra: HashMap<TestName, HashSet<StmtId>>,
}

impl Spectra {
    pub fn is_executed_in(&self, test: &TestName, stmt: &Statement) -> bool {
        if let Some(stmts) = self.spectra.get(test) {
            stmts.contains(&stmt.id)
        } else {
            false
        }
    }
}

impl<'data> FromRawData<'data> for Spectra {
    fn from_raw(data: &'data RawData, _api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let mut spectra = HashMap::new();
        let mut stmts = HashSet::new();
        let mut test_case = None;

        for item in data.trace.trace.iter() {
            match item {
                TraceItem::Test(name) => {
                    if !stmts.is_empty() && test_case.is_some() {
                        spectra.insert(test_case.unwrap(), stmts.clone());
                    }

                    test_case = Some(name.clone());
                    stmts.clear();
                }
                TraceItem::Statement(stmt) => {
                    stmts.insert(*stmt);
                }
                TraceItem::Value(_) => {} // Ignore
            }
        }

        if !stmts.is_empty() && test_case.is_some() {
            spectra.insert(test_case.unwrap(), stmts);
        }

        Ok(Spectra { spectra })
    }
}
