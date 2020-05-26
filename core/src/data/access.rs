//! Representation of variable access.

use std::fmt;

use crate::arena::{Arena, Dummy, DummyValue, P};

pub type VarId = u64;

/// The `Access` type represents an access to a variable. Such variable can be
/// either *scalar* (e.g., `foo`) or more complex, in particular *structural*
/// (e.g., `foo.bar`) and *array-like* (e.g., `foo[0]`). Complex accesses are
/// nested, so for example `foo[0].bar` is a structural access to array-like
/// access.
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

/// Implements a chain of raw variable/components identifiers specific for a
/// particular `Access`, such that it represents a *use* chain or *definition*
/// chain.
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct AccessChain(Vec<VarId>);

impl AccessChain {
    /// Creates new `AccessChain` for case of variable use.
    pub fn from_uses(access: &Access) -> Self {
        let mut uses = Vec::new();
        find_uses(access, &mut uses);
        AccessChain(uses)
    }

    /// Creates new `AccessChain` for case of variable definition.
    pub fn from_defs(access: &Access) -> Self {
        let mut defs = Vec::new();
        find_defs(access, &mut defs);
        AccessChain(defs)
    }

    /// Determines if the access chain (presumably "variable use" chain) is
    /// influenced by given definition access chain in the sense of data-flow
    /// analysis.
    pub fn influenced_by(&self, def_access: &AccessChain) -> bool {
        // A use is influenced by a definition if their access trees share the
        // same prefix. For instance:
        //
        // foo.bar = def()
        // use foo -> influenced
        // use foo.bar.baz -> influenced
        // use foo.quo -> not influenced

        def_access
            .iter()
            .zip(self.iter())
            .all(|(lhs, rhs)| lhs == rhs)
    }

    /// Iterates over raw variable/components identifiers.
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
