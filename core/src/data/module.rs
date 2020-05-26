//! Data related to static analysis.

use std::collections::HashMap;

use super::statement::Statement;
use super::types::{FileId, FileName, FuncName, StmtId};
use crate::arena::{P, S};

/// Data from all modules of the program.
pub struct Modules {
    /// Mapping from function names to the collection of its statements. The
    /// collection is actually a mapping from the statement global identifier to
    /// its full structure.
    pub functions: HashMap<S<FuncName>, HashMap<StmtId, P<Statement>>>,

    /// Mapping from file identifiers to their absolute paths.
    pub files: HashMap<FileId, S<FileName>>,
}

impl Modules {
    /// Initializes empty data.
    pub(crate) fn new() -> Self {
        Modules {
            functions: HashMap::new(),
            files: HashMap::new(),
        }
    }
}
