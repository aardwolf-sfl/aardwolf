use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::api::Api;
use crate::raw::data::{Access, Data, Statement, StmtId};
use crate::structures::{FromRawData, FromRawDataError};

struct DefUseItem<'data> {
    defs: HashSet<&'data Access>,
    uses: HashSet<&'data Access>,
}

pub struct DefUse<'data>(HashMap<StmtId, DefUseItem<'data>>);

impl<'data> DefUse<'data> {
    pub fn get_defs(&'data self, stmt: &Statement) -> Option<&'data HashSet<&'data Access>> {
        self.0.get(&stmt.id).map(|item| &item.defs)
    }

    pub fn get_uses(&'data self, stmt: &Statement) -> Option<&'data HashSet<&'data Access>> {
        self.0.get(&stmt.id).map(|item| &item.uses)
    }
}

impl<'data> FromRawData<'data> for DefUse<'data> {
    fn from_raw(data: &'data Data, _api: &'data Api<'data>) -> Result<Self, FromRawDataError> {
        let mut result = HashMap::new();

        for (_, func_body) in data.static_data.functions.iter() {
            for (id, stmt) in func_body.iter() {
                result.insert(
                    *id,
                    DefUseItem {
                        defs: HashSet::from_iter(stmt.defs.iter()),
                        uses: HashSet::from_iter(stmt.uses.iter()),
                    },
                );
            }
        }

        Ok(DefUse(result))
    }
}
