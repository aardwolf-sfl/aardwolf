mod cli;
mod json;

use std::fmt;

use crate::api::Api;
use crate::plugins::LocalizationItem;

pub use cli::CliUi;
pub use json::JsonUi;

pub trait Ui<'data> {
    fn prolog(&mut self) {}
    fn plugin(&mut self, id: &str);
    fn result(&mut self, item: &LocalizationItem);
    fn epilog(&mut self) {}
}

#[derive(Clone, Copy)]
pub enum UiName {
    Cli,
    Json,
}

impl Default for UiName {
    fn default() -> Self {
        UiName::Cli
    }
}

pub trait UiDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, api: &Api) -> fmt::Result;
}
