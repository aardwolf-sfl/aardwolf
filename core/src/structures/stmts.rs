use std::collections::{
    hash_map::{HashMap, Keys, Values},
    HashSet,
};

use crate::api::Api;
use crate::arena::{S, P};
use crate::data::{
    statement::Statement,
    trace::TraceItem,
    types::{FuncName, StmtId},
    RawData,
};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Stmts {
    mapping: HashMap<StmtId, P<Statement>>,
    functions: HashMap<StmtId, S<FuncName>>,
    n_total: usize,
    n_executed: usize,
}

impl Stmts {
    pub fn iter_ids(&self) -> Keys<StmtId, P<Statement>> {
        self.mapping.keys()
    }

    pub fn iter_stmts(&self) -> Values<StmtId, P<Statement>> {
        self.mapping.values()
    }

    pub fn get(&self, id: &StmtId) -> Option<P<Statement>> {
        self.mapping.get(id).map(|stmt| *stmt)
    }

    pub fn find_fn(&self, stmt: &Statement) -> Option<S<FuncName>> {
        self.functions.get(&stmt.id).copied()
    }

    pub fn get_n_total(&self) -> usize {
        self.n_total
    }

    pub fn get_n_executed(&self) -> usize {
        self.n_executed
    }
}

impl FromRawData for Stmts {
    fn from_raw(data: &RawData, _api: &Api) -> Result<Self, FromRawDataError> {
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
                            functions.insert(*stmt, *name);
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
                    mapping.insert(*id, *stmt);
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
