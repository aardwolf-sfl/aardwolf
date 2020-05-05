use std::collections::{
    hash_map::{HashMap, Keys, Values},
    HashSet,
};

use super::Query;
use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{
    statement::Statement,
    trace::TraceItem,
    types::{FuncName, StmtId},
    RawData,
};

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

    pub fn get(&self, id: &StmtId) -> Option<&P<Statement>> {
        self.mapping.get(id)
    }

    pub fn find_fn(&self, id: &StmtId) -> Option<&S<FuncName>> {
        self.functions.get(id)
    }

    pub fn get_n_total(&self) -> usize {
        self.n_total
    }

    pub fn get_n_executed(&self) -> usize {
        self.n_executed
    }
}

impl Query for Stmts {
    type Error = ();
    type Args = ();

    fn init(data: &RawData, _args: &Self::Args, _api: &Api) -> Result<Self, Self::Error> {
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
                    mapping.insert(*id, stmt.clone());
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
