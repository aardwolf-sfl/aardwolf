use std::collections::{
    hash_map::{HashMap, Keys, Values},
    HashSet,
};

use crate::api::Api;
use crate::raw::data::{Data, Statement, StaticData, TraceItem};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Stmts<'data> {
    mapping: HashMap<u64, &'data Statement>,
    raw: &'data StaticData,
}

impl<'data> Stmts<'data> {
    pub fn iter_ids(&self) -> Keys<u64, &'data Statement> {
        self.mapping.keys()
    }

    pub fn iter_stmts(&self) -> Values<u64, &'data Statement> {
        self.mapping.values()
    }

    pub fn get(&self, id: &u64) -> Option<&'data Statement> {
        self.mapping.get(id).map(|stmt| *stmt)
    }

    pub fn find_fn(&self, stmt: &Statement) -> Option<&'data String> {
        for (name, stmts) in self.raw.functions.iter() {
            if stmts.contains_key(&stmt.id) {
                return Some(name);
            }
        }

        None
    }
}

impl<'data> FromRawData<'data> for Stmts<'data> {
    fn from_raw(data: &'data Data, _api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
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
