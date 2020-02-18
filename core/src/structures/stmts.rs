use std::collections::{
    hash_map::{HashMap, Keys, Values},
    HashSet,
};

use crate::api::Api;
use crate::raw::data::{Data, Statement, StaticData, TraceItem};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Stmts<'a> {
    mapping: HashMap<u64, &'a Statement>,
    raw: &'a StaticData,
}

impl<'a> Stmts<'a> {
    pub fn iter_ids(&self) -> Keys<u64, &'a Statement> {
        self.mapping.keys()
    }

    pub fn iter_stmts(&self) -> Values<u64, &'a Statement> {
        self.mapping.values()
    }

    pub fn get(&self, id: &u64) -> Option<&'a Statement> {
        self.mapping.get(id).map(|stmt| *stmt)
    }

    pub fn find_fn(&self, stmt: &Statement) -> Option<&'a String> {
        for (name, stmts) in self.raw.functions.iter() {
            if stmts.contains_key(&stmt.id) {
                return Some(name);
            }
        }

        None
    }
}

impl<'a> FromRawData<'a> for Stmts<'a> {
    fn from_raw(data: &'a Data, _api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
        let mut executed = HashSet::new();
        let mut mapping = HashMap::new();

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
                    mapping.insert(*id, stmt);
                }
            }
        }

        Ok(Stmts {
            mapping,
            raw: &data.static_data,
        })
    }
}
