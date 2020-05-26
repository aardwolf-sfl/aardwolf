//! User interfaces for Aardwolf results presentation.

mod cli;
mod json;

use std::fmt;

use crate::api::Api;
use crate::plugins::{LocalizationItem, Metadata};

pub use cli::CliUi;
pub use json::JsonUi;

/// The trait implemented by all UIs.
pub trait Ui {
    /// Prepares for results presentation.
    fn prolog(&mut self, _api: &Api) {}
    /// Indicates that results for given plugin will be added.
    fn plugin(&mut self, id: &str, api: &Api);
    /// Presents the result as was determined by the plugin.
    fn result(&mut self, item: &LocalizationItem, api: &Api);
    /// Outputs the supplementary information provided with the results.
    fn metadata(&mut self, metadata: &Metadata, api: &Api);
    /// Finishes the results presentation.
    fn epilog(&mut self, _api: &Api) {}
    /// Displays the error encountered during the analysis.
    fn error(&mut self, error: &str);
}

/// Enumeration of available UIs.
#[derive(Clone, Copy)]
pub enum UiName {
    /// Command line interface UI. Currently the only actual UI in Aardwolf.
    Cli,
    /// JSON output that can be consumed by an external tool such as editor
    /// plugin.
    Json,
}

impl Default for UiName {
    fn default() -> Self {
        UiName::Cli
    }
}

/// Similar to [`Display`] in Rust's standard library. The only difference is
/// that it is provided with [`Api`] structure to be able to query the data.
///
/// [`Display`]: https://doc.rust-lang.org/nightly/std/fmt/trait.Display.html
/// [`Api`]: ../api/struct.Api.html
pub trait UiDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, api: &Api) -> fmt::Result;
}
