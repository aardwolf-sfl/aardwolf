use std::collections::{HashMap, HashSet};

use crate::api::Api;
use crate::raw::data::{Data, Statement, TestName, TraceItem};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Spectra<'a> {
    spectra: HashMap<&'a TestName, HashSet<u64>>,
}

impl<'a> Spectra<'a> {
    pub fn is_executed_in(&self, test: &TestName, stmt: &Statement) -> bool {
        if let Some(stmts) = self.spectra.get(test) {
            stmts.contains(&stmt.id)
        } else {
            false
        }
    }
}

impl<'a> FromRawData<'a> for Spectra<'a> {
    fn from_raw(data: &'a Data, _api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
        let mut spectra = HashMap::new();
        let mut stmts = HashSet::new();
        let mut test_case = None;

        for item in data.dynamic_data.trace.iter() {
            match item {
                TraceItem::External(name) => {
                    if !stmts.is_empty() && test_case.is_some() {
                        spectra.insert(test_case.unwrap(), stmts.clone());
                    }

                    test_case = Some(name);
                    stmts.clear();
                }
                TraceItem::Statement(stmt) => {
                    stmts.insert(*stmt);
                }
                TraceItem::Data(_) => {} // Ignore
            }
        }

        if !stmts.is_empty() && test_case.is_some() {
            spectra.insert(test_case.unwrap(), stmts);
        }

        Ok(Spectra { spectra })
    }
}
