use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;

use crate::api::Api;
use crate::raw::data::{Access, Data, Statement};
use crate::structures::{FromRawData, FromRawDataError};

struct DefUseItem<'a> {
    defs: HashSet<&'a Access>,
    uses: HashSet<&'a Access>,
}

pub struct DefUse<'a>(HashMap<u64, DefUseItem<'a>>);

impl<'a> DefUse<'a> {
    pub fn get_defs(&'a self, stmt: &Statement) -> Option<&'a HashSet<&'a Access>> {
        self.0.get(&stmt.id).map(|item| &item.defs)
    }

    pub fn get_uses(&'a self, stmt: &Statement) -> Option<&'a HashSet<&'a Access>> {
        self.0.get(&stmt.id).map(|item| &item.uses)
    }
}

impl<'a> FromRawData<'a> for DefUse<'a> {
    fn from_raw(data: &'a Data, _api: &'a Api<'a>) -> Result<Self, FromRawDataError> {
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
