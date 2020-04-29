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
    fn result(&mut self, item: &LocalizationItem<'data>);
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

pub trait UiDisplay<'data> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>, api: &'data Api<'data>) -> fmt::Result;
}
