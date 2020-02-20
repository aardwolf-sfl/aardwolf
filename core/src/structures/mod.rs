mod cfg;
mod def_use;
mod spectra;
mod stmts;
mod tests;

use crate::api::Api;
use crate::raw::data::Data;

pub use cfg::{Cfgs, Cfg, ENTRY, EXIT};
pub use def_use::DefUse;
pub use spectra::Spectra;
pub use stmts::Stmts;
pub use tests::Tests;

#[derive(Debug)]
pub enum FromRawDataError {
    Inner(String),
}

// Plugins can register their own high-level data structures that implement this trait.
// Implementing this trait is the only way how to access the raw data. Plugins themselves
// must use only registered high-level data structures. Data structure registration is
// separate call in plugin interface. This design pattern should enforce clean separation
// of concerns.
pub trait FromRawData<'a> {
    fn from_raw(data: &'a Data, api: &'a Api<'a>) -> Result<Self, FromRawDataError>
    where
        Self: Sized;
}
