mod cli;
mod json;

use std::fmt;

use crate::api::Api;
use crate::plugins::LocalizationItem;

pub use cli::CliUi;
pub use json::JsonUi;

pub trait Ui {
    fn prolog(&mut self, _api: &Api) {}
    fn plugin(&mut self, id: &str, api: &Api);
    fn result(&mut self, item: &LocalizationItem, api: &Api);
    fn epilog(&mut self, _api: &Api) {}
    fn error(&mut self, error: &str);
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
