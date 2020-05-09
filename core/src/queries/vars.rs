use std::mem;

use super::stmts::Stmts;
use super::{Query, QueryInitError};
use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{
    access::Access, statement::Statement, trace::TraceItem, types::TestName, values::Value, RawData,
};

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

pub struct Vars(Vec<VarItem>);

impl Vars {
    pub fn iter(&self) -> impl Iterator<Item = &VarItem> {
        self.0.iter()
    }
}

impl Query for Vars {
    type Error = QueryInitError;
    type Args = S<TestName>;

    fn init(data: &RawData, args: &Self::Args, api: &Api) -> Result<Self, Self::Error> {
        let stmts = api.query::<Stmts>()?;

        // We need a stack because of collecting definitions of calling statements.
        // But we put assignment statements into the stack as well
        // (which will be popped out right in the next iteration),
        // just not to complicate things.
        let mut stack = Vec::new();
        let mut defs = Vec::new();

        let mut vars = Vec::new();
        let mut non_empty = false;

        for item in data.trace.find_test(args) {
            match item {
                TraceItem::Test(_) => {}
                TraceItem::Statement(id) => {
                    // Stmts are built from dynamic trace so a statement with this id certainly exists.
                    let stmt_ptr = stmts.get(id).unwrap();
                    let stmt = stmt_ptr.as_ref();

                    if !stmt.defs.is_empty() {
                        stack.push((*stmt_ptr, stmt));
                    }

                    non_empty = true;
                }
                TraceItem::Value(value) => {
                    defs.push(*value);

                    if let Some((stmt_ptr, stmt)) = stack.last() {
                        // We collected all definitions of the last statement.
                        if stmt.defs.len() == defs.len() {
                            vars.push(VarItem {
                                stmt: *stmt_ptr,
                                defs: mem::take(&mut defs),
                            });
                            stack.pop();
                        }
                    } else {
                        return Err(QueryInitError::Custom(Box::new(format!(
                            "Invalid trace file."
                        ))));
                    }
                }
            }
        }

        if vars.is_empty() {
            if non_empty {
                Err(QueryInitError::Custom(Box::new(format!(
                    "Missing variable trace."
                ))))
            } else {
                Err(QueryInitError::InvalidTestName(args.clone()))
            }
        } else {
            Ok(Vars(vars))
        }
    }
}
