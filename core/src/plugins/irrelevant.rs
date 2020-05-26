use std::collections::{HashMap, HashSet};

use yaml_rust::Yaml;

use crate::api::Api;
use crate::data::{types::StmtId, RawData};
use crate::plugins::{AardwolfPlugin, PluginError, PluginInitError, Preprocessing};
use crate::queries::{Query, Tests};

pub struct Irrelevant;

impl AardwolfPlugin for Irrelevant {
    fn init(_api: &Api, _opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Irrelevant)
    }

    fn run_pre(&self, api: &Api, preprocessing: &mut Preprocessing) -> Result<(), PluginError> {
        let failing = api.query::<FailingStmts>()?;

        preprocessing.set_stmt_priorities(
            |stmt, prio| {
                if failing.contains(stmt) {
                    0.0
                } else {
                    prio
                }
            },
        );

        Ok(())
    }
}

struct FailingStmts(HashSet<StmtId>);

impl FailingStmts {
    pub fn contains(&self, stmt: &StmtId) -> bool {
        self.0.contains(stmt)
    }
}

impl Query for FailingStmts {
    type Error = ();
    type Args = ();

    fn init(_data: &RawData, _args: &Self::Args, api: &Api) -> Result<Self, Self::Error> {
        let tests = api.query::<Tests>()?;
        let mut executed = HashSet::new();

        for test in tests.iter_failed() {
            for stmt in tests.iter_stmts(test).unwrap() {
                executed.insert(stmt.as_ref().id);
            }
        }

        Ok(FailingStmts(executed))
    }
}
