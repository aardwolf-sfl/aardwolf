use std::collections::{HashMap, HashSet};

use yaml_rust::Yaml;

use crate::api::Api;
use crate::plugins::{AardwolfPlugin, IrrelevantItems, PluginError, PluginInitError};
use crate::raw::data::{Data, Statement};
use crate::structures::{FromRawData, FromRawDataError};

pub struct Irrelevant;

impl AardwolfPlugin for Irrelevant {
    fn init<'data>(
        _api: &'data Api<'data>,
        _opts: &HashMap<String, Yaml>,
    ) -> Result<Self, PluginInitError>
    where
        Self: Sized,
    {
        Ok(Irrelevant)
    }

    fn run_pre<'data, 'out>(
        &'out self,
        api: &'data Api<'data>,
        irrelevant: &'out mut IrrelevantItems<'data>,
    ) -> Result<(), PluginError> {
        let failing = api.make::<FailingStmts>().unwrap();
        let stmts = api.get_stmts();

        for stmt in stmts.iter_stmts() {
            if !failing.contains(stmt) {
                irrelevant.mark_stmt(stmt);
            }
        }

        Ok(())
    }
}

struct FailingStmts<'data>(HashSet<&'data Statement>);

impl<'data> FailingStmts<'data> {
    pub fn contains(&self, stmt: &'data Statement) -> bool {
        self.0.contains(stmt)
    }
}

impl<'data> FromRawData<'data> for FailingStmts<'data> {
    fn from_raw(_data: &'data Data, api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let tests = api.get_tests();
        let mut executed = HashSet::new();

        for test in tests.iter_failed() {
            for stmt in tests.iter_stmts(test).unwrap() {
                executed.insert(stmt);
            }
        }

        Ok(FailingStmts(executed))
    }
}
