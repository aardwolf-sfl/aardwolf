use std::collections::{
    hash_map::{HashMap, Keys, Values},
    HashSet,
};

use crate::api::Api;
use crate::data::{
    statement::Statement,
    trace::TraceItem,
    types::{FuncName, StmtId},
    RawData,
};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Stmts<'data> {
    mapping: HashMap<StmtId, &'data Statement>,
    functions: HashMap<StmtId, FuncName>,
    n_total: usize,
    n_executed: usize,
}

impl<'data> Stmts<'data> {
    pub fn iter_ids(&self) -> Keys<StmtId, &'data Statement> {
        self.mapping.keys()
    }

    pub fn iter_stmts(&self) -> Values<StmtId, &'data Statement> {
        self.mapping.values()
    }

    pub fn get(&self, id: &StmtId) -> Option<&'data Statement> {
        self.mapping.get(id).map(|stmt| *stmt)
    }

    pub fn find_fn(&self, stmt: &Statement) -> Option<&FuncName> {
        self.functions.get(&stmt.id)
    }

    pub fn get_n_total(&self) -> usize {
        self.n_total
    }

    pub fn get_n_executed(&self) -> usize {
        self.n_executed
    }
}

impl<'data> FromRawData<'data> for Stmts<'data> {
    fn from_raw(data: &'data RawData, _api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let mut executed = HashSet::new();
        let mut mapping = HashMap::new();
        let mut functions = HashMap::new();

        let mut n_total = 0;
        let mut n_executed = 0;

        for item in data.trace.trace.iter() {
            match item {
                TraceItem::Statement(stmt) => {
                    executed.insert(*stmt);

                    for (name, stmts) in data.modules.functions.iter() {
                        if stmts.contains_key(stmt) {
                            functions.insert(*stmt, name.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        for (_, stmts) in data.modules.functions.iter() {
            for (id, stmt) in stmts.iter() {
                n_total += 1;

                if executed.contains(id) {
                    mapping.insert(*id, stmt);
                    n_executed += 1;
                }
            }
        }

        Ok(Stmts {
            mapping,
            functions,
            n_total,
            n_executed,
        })
    }
}
