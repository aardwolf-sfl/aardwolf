use std::collections::{
    hash_map::{HashMap, Keys, Values},
    HashSet,
};

use crate::api::Api;
use crate::raw::data::{Data, Statement, TraceItem};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Stmts<'a>(HashMap<u64, &'a Statement>);

impl<'a> Stmts<'a> {
    pub fn iter_ids(&self) -> Keys<u64, &'a Statement> {
        self.0.keys()
    }

    pub fn iter_stmts(&self) -> Values<u64, &'a Statement> {
        self.0.values()
    }
}

impl<'a> FromRawData<'a> for Stmts<'a> {
    fn from_raw(data: &'a Data, _api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
        let mut executed = HashSet::new();
        let mut result = HashMap::new();

        for item in data.dynamic_data.trace.iter() {
            match item {
                TraceItem::Statement(stmt) => {
                    executed.insert(*stmt);
                }
                _ => {}
            }
        }

        for (_, stmts) in data.static_data.functions.iter() {
            for (id, stmt) in stmts.iter() {
                if executed.contains(id) {
                    result.insert(*id, stmt);
                }
            }
        }

        Ok(Stmts(result))
    }
}
