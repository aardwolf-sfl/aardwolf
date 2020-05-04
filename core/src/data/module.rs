use std::collections::HashMap;

use super::statement::Statement;
use super::types::{FileId, FileName, FuncName, StmtId};
use crate::arena::{S, P};

pub struct Modules {
    pub functions: HashMap<S<FuncName>, HashMap<StmtId, P<Statement>>>,
    pub files: HashMap<FileId, S<FileName>>,
}

impl Modules {
    pub(crate) fn new() -> Self {
        Modules {
            functions: HashMap::new(),
            files: HashMap::new(),
        }
    }
}
