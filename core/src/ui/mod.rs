mod cli;
mod json;

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
