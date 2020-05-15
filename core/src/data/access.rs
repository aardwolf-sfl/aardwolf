use std::fmt;

use crate::arena::{Arena, Dummy, DummyValue, P};

pub type VarId = u64;

#[derive(Hash, PartialEq, Eq)]
pub enum Access {
    Scalar(VarId),
    Structural(Box<Access>, Box<Access>),
    ArrayLike(Box<Access>, Vec<Access>),
}

impl fmt::Debug for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Access::Scalar(id) => write!(f, "%{}", id),
            Access::Structural(base, field) => write!(f, "{:?}.{:?}", base, field),
            Access::ArrayLike(base, index) => {
                if index.is_empty() {
                    write!(f, "{:?}[]", base)
                } else {
                    write!(f, "{:?}[{:?}", base, index[0])?;
                    for item in index.iter().skip(1) {
                        write!(f, ", {:?}", item)?;
                    }
                    write!(f, "]")
                }
            }
        }
    }
}

impl DummyValue for Access {
    fn dummy(dummy: Dummy) -> Self {
        Access::Scalar(VarId::MAX - (dummy.as_num() as VarId))
    }
}

impl_arena_type!(P<Access>, Arena<Access>);

impl P<Access> {
    pub fn as_ref(&self) -> &Access {
        Self::arena().get(self)
    }
}

#[derive(Hash, PartialEq, Eq)]
pub struct AccessChain(Vec<VarId>);

impl AccessChain {
    pub fn from_uses(access: &Access) -> Self {
        let mut uses = Vec::new();
        find_uses(access, &mut uses);
        AccessChain(uses)
    }

    pub fn from_defs(access: &Access) -> Self {
        let mut defs = Vec::new();
        find_defs(access, &mut defs);
        AccessChain(defs)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, VarId> {
        self.0.iter()
    }
}

fn find_uses(access: &Access, uses: &mut Vec<VarId>) {
    match access {
        Access::Scalar(id) => uses.push(*id),
        Access::Structural(base, field) => {
            find_uses(base, uses);
            find_uses(field, uses);
        }
        Access::ArrayLike(base, index) => {
            find_uses(base, uses);
            for idx in index {
                find_uses(idx, uses);
            }
        }
    }
}

fn find_defs(access: &Access, defs: &mut Vec<VarId>) {
    match access {
        Access::Scalar(id) => defs.push(*id),
        Access::Structural(base, field) => {
            find_defs(base, defs);
            find_defs(field, defs);
        }
        Access::ArrayLike(base, _) => {
            find_defs(base, defs);
            // Do not use index variables.
        }
    }
}

impl IntoIterator for AccessChain {
    type Item = VarId;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
