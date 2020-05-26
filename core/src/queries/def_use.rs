//! Def-use sets in a function.

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use super::{Query, QueryInitError};
use crate::api::Api;
use crate::arena::{P, S};
use crate::data::{
    access::Access,
    statement::Statement,
    types::{FuncName, StmtId},
    RawData,
};

struct DefUseItem {
    defs: HashSet<P<Access>>,
    uses: HashSet<P<Access>>,
}

pub struct DefUse(HashMap<StmtId, DefUseItem>);

impl DefUse {
    /// Gets the variables defined by given statement. The statement must be
    /// from the function for which the def-use set was computed.
    pub fn get_defs(&self, stmt: &Statement) -> Option<&HashSet<P<Access>>> {
        self.0.get(&stmt.id).map(|item| &item.defs)
    }

    /// Gets the variables used by given statement. The statement must be from
    /// the function for which the def-use set was computed.
    pub fn get_uses(&self, stmt: &Statement) -> Option<&HashSet<P<Access>>> {
        self.0.get(&stmt.id).map(|item| &item.uses)
    }
}

impl Query for DefUse {
    type Error = QueryInitError;
    type Args = S<FuncName>;

    fn init(data: &RawData, args: &Self::Args, _api: &Api) -> Result<Self, Self::Error> {
        let mut result = HashMap::new();

        data.modules
            .functions
            .iter()
            .find(|(func_name, _)| *func_name == args)
            .ok_or(QueryInitError::InvalidFuncName(*args))
            .map(|(_, func_body)| {
                for (id, stmt) in func_body.iter() {
                    let stmt = stmt.as_ref();

                    result.insert(
                        *id,
                        DefUseItem {
                            defs: HashSet::from_iter(stmt.defs.iter().copied()),
                            uses: HashSet::from_iter(stmt.uses.iter().copied()),
                        },
                    );
                }

                DefUse(result)
            })
    }
}
