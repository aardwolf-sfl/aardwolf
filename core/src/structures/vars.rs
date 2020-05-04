use std::collections::hash_map::HashMap;
use std::mem;

use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{
    access::Access, statement::Statement, trace::TraceItem, types::TestName, values::Value, RawData,
};
use crate::structures::{FromRawData, FromRawDataError};

#[derive(Debug)]
pub struct VarItem {
    pub stmt: P<Statement>,
    pub defs: Vec<P<Value>>,
}

impl VarItem {
    pub fn zip(&self) -> impl Iterator<Item = (&P<Access>, &P<Value>)> {
        self.stmt.as_ref().defs.iter().zip(self.defs.iter())
    }
}

pub struct Vars {
    traces: HashMap<S<TestName>, Vec<VarItem>>,
}

impl Vars {
    pub fn iter_vars(&self, test: &S<TestName>) -> Option<impl Iterator<Item = &VarItem>> {
        self.traces.get(test).map(|stmts| stmts.iter())
    }
}

impl FromRawData for Vars {
    fn from_raw(data: &RawData, api: &Api) -> Result<Self, FromRawDataError> {
        let stmts = api.get_stmts();

        let mut traces = HashMap::with_capacity(data.test_suite.tests.len());

        let mut test = None;
        let mut trace = Vec::new();

        // We need a stack because of collecting definitions of calling statements.
        // But we put assignment statements into the stack as well
        // (which will be popped out right in the next iteration),
        // just not to complicate things.
        let mut stack = Vec::new();
        let mut defs = Vec::new();

        for item in data.trace.trace.iter() {
            match item {
                TraceItem::Statement(id) => {
                    // Stmts are built from dynamic trace so a statement with this id certainly exists.
                    let stmt = stmts.get(id).unwrap();
                    if !stmt.as_ref().defs.is_empty() {
                        stack.push(stmt);
                    }
                }
                TraceItem::Test(new_test) => {
                    if let Some(test) = test {
                        // Insert the trace and clear reset the trace variable in one step.
                        traces.insert(test, mem::take(&mut trace));
                    } else {
                        // Clear the trace when when it is not empty
                        // as we don't have a test to associate the variables with anyway.
                        traces.clear();
                    }

                    test = Some(new_test.clone());
                }
                TraceItem::Value(value) => {
                    defs.push(*value);

                    if let Some(stmt) = stack.last() {
                        // We collected all definitions of the last statement.
                        if stmt.as_ref().defs.len() == defs.len() {
                            trace.push(VarItem {
                                stmt: *stmt,
                                defs: mem::take(&mut defs),
                            });
                            stack.pop();
                        }
                    } else {
                        return Err(FromRawDataError::Inner(format!("Invalid trace file.")));
                    }
                }
            }
        }

        // Insert the variables that remain.
        if let Some(test) = test {
            traces.insert(test, mem::take(&mut trace));
        }

        Ok(Vars { traces })
    }
}
