use std::collections::{HashMap, HashSet};

use yaml_rust::Yaml;

use crate::api::Api;
use crate::arena::P;
use crate::data::{statement::Statement, RawData};
use crate::plugins::{AardwolfPlugin, IrrelevantItems, PluginError, PluginInitError};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Irrelevant;

impl AardwolfPlugin for Irrelevant {
    fn init<'data>(_api: &'data Api, _opts: &HashMap<String, Yaml>) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Irrelevant)
    }

    fn run_pre<'data, 'out>(
        &'out self,
        api: &'data Api,
        irrelevant: &'out mut IrrelevantItems,
    ) -> Result<(), PluginError> {
        let failing = api.make::<FailingStmts>().unwrap();
        let stmts = api.get_stmts();

        for stmt_ptr in stmts.iter_stmts() {
            let stmt = stmt_ptr.as_ref();
            if !failing.contains(stmt_ptr) {
                irrelevant.mark_stmt(stmt);
            }
        }

        Ok(())
    }
}

struct FailingStmts(HashSet<P<Statement>>);

impl FailingStmts {
    pub fn contains(&self, stmt: &P<Statement>) -> bool {
        self.0.contains(stmt)
    }
}

impl FromRawData for FailingStmts {
    fn from_raw(_data: &RawData, api: &Api) -> Result<Self, FromRawDataError> {
        let tests = api.get_tests();
        let mut executed = HashSet::new();

        for test in tests.iter_failed() {
            for stmt in tests.iter_stmts(test).unwrap() {
                executed.insert(*stmt);
            }
        }

        Ok(FailingStmts(executed))
    }
}
