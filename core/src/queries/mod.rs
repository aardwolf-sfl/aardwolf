pub mod cfg;
pub mod def_use;
pub mod pdg;
pub mod spectra;
pub mod stmts;
pub mod tests;
pub mod vars;

use std::fmt;
use std::hash::Hash;

use crate::api::Api;
use crate::arena::S;
use crate::data::{
    types::{FuncName, TestName},
    RawData,
};

pub use cfg::Cfg;
pub use def_use::DefUse;
pub use pdg::Pdg;
pub use spectra::Spectra;
pub use stmts::Stmts;
pub use tests::Tests;
pub use vars::Vars;

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum QueryKey {
    FuncName(S<FuncName>),
    TestName(S<TestName>),
    None,
}

pub trait QueryArgs {
    fn key(&self) -> QueryKey;
}

impl QueryArgs for S<FuncName> {
    fn key(&self) -> QueryKey {
        QueryKey::FuncName(*self)
    }
}

impl QueryArgs for S<TestName> {
    fn key(&self) -> QueryKey {
        QueryKey::TestName(*self)
    }
}

impl QueryArgs for () {
    fn key(&self) -> QueryKey {
        QueryKey::None
    }
}

// Query is intended to provide high-level interface over raw data which should
// not be used in fault localization plugins directly. User is encouraged to
// implement their own queries.
//
// Needs to be Sized due to use in Result. Needs to be 'static in order to allow
// conversion to Any.
pub trait Query: Sized + 'static {
    type Error;
    type Args: QueryArgs;

    fn init(data: &RawData, args: &Self::Args, api: &Api) -> Result<Self, Self::Error>;
}

#[derive(Debug)]
pub enum QueryInitError {
    Custom(Box<dyn fmt::Debug>),
    InvalidFuncName(S<FuncName>),
    InvalidTestName(S<TestName>),
}

// This implementation allows to use `?` when getting a unit error returning
// query in queries that return QueryInitError.
impl From<()> for QueryInitError {
    fn from(_err: ()) -> Self {
        // We assume that when the error type is unit type, then such error is
        // never returned.
        unreachable!()
    }
}
