use std::collections::hash_map::HashMap;
use std::mem;

use crate::api::Api;
use crate::raw::data::{Access, Data, Statement, TestName, TraceItem, VariableData};
use crate::structures::{FromRawData, FromRawDataError};

#[derive(Debug)]
pub struct VarItem<'data> {
    pub stmt: &'data Statement,
    pub defs: Vec<VariableData>,
    // TODO:
    // pub uses: Vec<VariableData>,
}

impl<'data> VarItem<'data> {
    pub fn zip(&self) -> impl Iterator<Item = (&Access, &VariableData)> {
        self.stmt.defs.iter().zip(self.defs.iter())
    }
}

pub struct Vars<'data> {
    traces: HashMap<&'data TestName, Vec<VarItem<'data>>>,
}

impl<'data> Vars<'data> {
    pub fn iter_vars(
        &'data self,
        test: &'data TestName,
    ) -> Option<impl Iterator<Item = &'data VarItem<'data>>> {
        self.traces.get(test).map(|stmts| stmts.iter())
    }
}

impl<'data> FromRawData<'data> for Vars<'data> {
    fn from_raw(data: &'data Data, api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let stmts = api.get_stmts();

        let mut traces = HashMap::with_capacity(data.test_data.tests.len());

        let mut test = None;
        let mut trace = Vec::new();

        // We need a stack because of collecting definitions of calling statements.
        // But we put assignment statements into the stack as well
        // (which will be popped out right in the next iteration),
        // just not to complicate things.
        let mut stack = Vec::new();
        let mut defs = Vec::new();

        for item in data.dynamic_data.trace.iter() {
            match item {
                TraceItem::Statement(id) => {
                    // Stmts are built from dynamic trace so a statement with this id certainly exists.
                    let stmt = stmts.get(id).unwrap();
                    if !stmt.defs.is_empty() {
                        stack.push(stmt);
                    }
                }
                TraceItem::External(new_test) => {
                    if let Some(test) = test {
                        // Insert the trace and clear reset the trace variable in one step.
                        traces.insert(test, mem::take(&mut trace));
                    } else {
                        // Clear the trace when when it is not empty
                        // as we don't have a test to associate the variables with anyway.
                        traces.clear();
                    }

                    test = Some(new_test);
                }
                TraceItem::Data(data) => {
                    defs.push(*data);

                    if let Some(stmt) = stack.last() {
                        // We collected all definitions of the last statement.
                        if stmt.defs.len() == defs.len() {
                            trace.push(VarItem {
                                stmt,
                                defs: mem::take(&mut defs),
                            });
                            stack.pop();
                        }
                    } else {
                        // TODO: Return Err(...)
                        panic!("invalid input");
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
