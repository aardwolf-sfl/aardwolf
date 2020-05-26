//! Several utilities for statements in the whole program.

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
    n_total: usize,
    n_executed: usize,
}

impl Stmts {
    /// Iterates over all statement identifiers in the program.
    pub fn iter_ids(&self) -> Keys<StmtId, P<Statement>> {
        self.mapping.keys()
    }

    /// Iterates over all statements in the program.
    pub fn iter_stmts(&self) -> Values<StmtId, P<Statement>> {
        self.mapping.values()
    }

    /// Gets a statement by its identifier.
    pub fn get(&self, id: &StmtId) -> Option<&P<Statement>> {
        self.mapping.get(id)
    }

    /// Gets the function name where the statement is located in.
    pub fn find_fn(&self, id: &StmtId) -> Option<&S<FuncName>> {
        self.get(id).map(|stmt| &stmt.as_ref().func)
    }

    /// Gets the total number of statements in the program.
    pub fn get_n_total(&self) -> usize {
        self.n_total
    }

    /// Gets the total number of *executed* statements in the program.
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

        let mut n_total = 0;
        let mut n_executed = 0;

        for item in data.trace.trace.iter() {
            match item {
                TraceItem::Statement(stmt) => {
                    executed.insert(*stmt);
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
            n_total,
            n_executed,
        })
    }
}
