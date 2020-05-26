//! High-level queries to raw data.
//!
//! Working with raw data is cumbersome. Aardwolf therefore implements a query
//! system where queries implement a high-level interface on top of the data
//! providing convenient interface for the user. We encourage the user to
//! implement their custom data queries.
//!
//! Each query can have a single argument to limit its scope. Parameter-less
//! queries just have `()` as their argument. Currently, the list of possible
//! arguments is limited to function name, test name and none.
//!
//! The data queries are lazily evaluated and memoized in [`Api`] structure. The
//! memoization key is constructed from query's type id and the argument value.
//!
//! [`Api`]: ../api/struct.Api.html

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

/// Query key which represents its argument. This key is used as one of the
/// components when memoizing the query result.
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum QueryKey {
    FuncName(S<FuncName>),
    TestName(S<TestName>),
    None,
}

/// An argument of a query must implement this trait which specifies the key by
/// which its memoized.
pub trait QueryArgs {
    /// Returns [`QueryKey`] which is used as one of the components when
    /// memoizing the query result.
    ///
    /// [`QueryKey`]: enum.QueryKey.html
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

/// Query is intended to provide high-level interface over raw data which should
/// not be used in fault localization plugins directly. User is encouraged to
/// implement their own queries.
pub trait Query: Sized + 'static {
    // The trait needs to be Sized due to use in Result and needs to be 'static
    // in order to allow conversion to Any.

    /// Error type used when the query fails.
    type Error;
    /// Argument type for the query.
    type Args: QueryArgs;

    /// Executes the query on the raw data. [`Api`] structure is provided so
    /// other queries can be called as dependencies. The query execution cn
    /// fail.
    ///
    /// [`Api`]: ../api/struct.Api.html
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
