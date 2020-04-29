mod cfg;
mod def_use;
pub(crate) mod pdg;
mod spectra;
mod stmts;
mod tests;
mod vars;

use crate::api::Api;
use crate::data::RawData;

pub use cfg::{Cfg, Cfgs, ENTRY, EXIT};
pub use def_use::DefUse;
pub use pdg::{EdgeType, Pdg, Pdgs};
pub use spectra::Spectra;
pub use stmts::Stmts;
pub use tests::Tests;
pub use vars::{VarItem, Vars};

#[derive(Debug)]
pub enum FromRawDataError {
    Inner(String),
}

// Plugins can register their own high-level data queries that implement this
// trait. Implementing this trait is the only way how to access the raw data.
// Plugins themselves must use only registered high-level data queries. Data
// structure registration is separate call in plugin interface. This design
// pattern should enforce clean separation of concerns.
pub trait FromRawData<'data> {
    fn from_raw(data: &'data RawData, api: &'data Api<'data>) -> Result<Self, FromRawDataError>
    where
        Self: Sized;
}
