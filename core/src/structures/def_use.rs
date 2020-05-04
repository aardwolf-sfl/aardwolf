use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::api::Api;
use crate::arena::P;
use crate::data::{access::Access, statement::Statement, types::StmtId, RawData};
use crate::structures::{FromRawData, FromRawDataError};

struct DefUseItem {
    defs: HashSet<P<Access>>,
    uses: HashSet<P<Access>>,
}

pub struct DefUse(HashMap<StmtId, DefUseItem>);

impl DefUse {
    pub fn get_defs(&self, stmt: &Statement) -> Option<&HashSet<P<Access>>> {
        self.0.get(&stmt.id).map(|item| &item.defs)
    }

    pub fn get_uses(&self, stmt: &Statement) -> Option<&HashSet<P<Access>>> {
        self.0.get(&stmt.id).map(|item| &item.uses)
    }
}

impl FromRawData for DefUse {
    fn from_raw(data: &RawData, _api: &Api) -> Result<Self, FromRawDataError> {
        let mut result = HashMap::new();

        for (_, func_body) in data.modules.functions.iter() {
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
        }

        Ok(DefUse(result))
    }
}
