use std::collections::HashMap;

use super::statement::Statement;
use super::types::{FileId, FuncName, StmtId};

pub struct Modules {
    pub functions: HashMap<FuncName, HashMap<StmtId, Statement>>,
    pub files: HashMap<FileId, String>,
}

impl Modules {
    pub(crate) fn new() -> Self {
        Modules {
            functions: HashMap::new(),
            files: HashMap::new(),
        }
    }
}
