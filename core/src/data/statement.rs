//! Data related to individual statements.

use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};

use super::access::Access;
use super::consts;
use super::types::{FileId, FuncName, StmtId};
use crate::arena::{Arena, Dummy, DummyValue, P, S};

/// Location information.
///
/// It contains file identifier, which can be used to obtain absolute path to
/// the file, and line and column information about the statement beginning and
/// end in the source code.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Loc {
    pub file_id: FileId,
    pub line_begin: u32,
    pub col_begin: u32,
    pub line_end: u32,
    pub col_end: u32,
}

impl Loc {
    /// Merges two location together producing a new one which spans over both
    /// original ones.
    pub fn merge(&self, other: &Self) -> Self {
        let line_begin = u32::min(self.line_begin, other.line_begin);
        let col_begin = u32::min(self.col_begin, other.col_begin);
        let line_end = u32::max(self.line_end, other.line_end);
        let col_end = u32::max(self.col_end, other.col_end);
        Loc {
            file_id: self.file_id,
            line_begin,
            col_begin,
            line_end,
            col_end,
        }
    }

    /// Determines whether given location is inside the `self` location.
    pub fn contains(&self, other: &Self) -> bool {
        let file = self.file_id == other.file_id;

        let begin = if self.line_begin == other.line_begin {
            self.col_begin <= other.col_begin
        } else {
            self.line_begin <= other.line_begin
        };

        let end = if self.line_end == other.line_end {
            self.col_end >= other.col_end
        } else {
            self.line_end >= other.line_end
        };

        file && begin && end
    }
}

impl fmt::Debug for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "@{} {}:{}-{}:{}",
            self.file_id, self.line_begin, self.col_begin, self.line_end, self.col_end
        )
    }
}

impl DummyValue for Loc {
    fn dummy(dummy: Dummy) -> Self {
        Loc {
            file_id: FileId::dummy(dummy),
            line_begin: 0,
            col_begin: 0,
            line_end: 0,
            col_end: 0,
        }
    }
}

/// High-level structure for accessing statement metadata encoded in a byte
/// form.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Metadata(u8);

impl Metadata {
    pub(crate) const fn new(byte: u8) -> Self {
        Metadata(byte)
    }

    /// Indicates whether the statement is a function argument.
    pub fn is_arg(&self) -> bool {
        self.is_meta(consts::META_ARG)
    }

    /// Indicates whether the statement is a return from a function.
    pub fn is_ret(&self) -> bool {
        self.is_meta(consts::META_RET)
    }

    /// Indicates whether the statement is a call to a function.
    pub fn is_call(&self) -> bool {
        self.is_meta(consts::META_CALL)
    }

    /// Determines if the statement has no metadata assigned.
    pub fn empty(&self) -> bool {
        self.0 == 0
    }

    fn is_meta(&self, cst: u8) -> bool {
        let meta = consts::META;
        (self.0 & !meta) == (cst & !meta)
    }
}

impl DummyValue for Metadata {
    fn dummy(_dummy: Dummy) -> Self {
        Metadata(0)
    }
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.empty() {
            write!(f, "  {{ ")?;

            if self.is_arg() {
                write!(f, "arg")?;
            }

            if self.is_ret() {
                if self.is_arg() {
                    write!(f, ", ")?;
                }

                write!(f, "ret")?;
            }

            write!(f, " }}")?;
        }

        Ok(())
    }
}

/// Structure representing the statement in the program.
pub struct Statement {
    /// Globally unique identifier.
    pub id: StmtId,
    /// Statement's successors in the control flow graph.
    pub succ: Vec<StmtId>,
    /// List of variable accesses the statement defines.
    pub defs: Vec<P<Access>>,
    /// List of variable accesses the statement uses.
    pub uses: Vec<P<Access>>,
    /// Statement's location in the source code.
    pub loc: Loc,
    /// Statement's metadata.
    pub metadata: Metadata,
    /// The function name in which the statement resides.
    pub func: S<FuncName>,
}

impl Statement {
    /// Determines whether the statement is predicate node. It does so by
    /// checking the number of successors.
    pub fn is_predicate(&self) -> bool {
        self.succ.len() > 1
    }

    /// Determines if the given statement is `self`'s successor.
    pub fn is_succ(&self, stmt: &Statement) -> bool {
        self.succ.iter().any(|succ| succ == &stmt.id)
    }

    #[cfg(test)]
    pub fn new_test(stmt_id: StmtId) -> Self {
        Statement {
            id: stmt_id,
            succ: Vec::new(),
            defs: Vec::new(),
            uses: Vec::new(),
            loc: Loc::dummy(Dummy::D1),
            metadata: Metadata::dummy(Dummy::D1),
            func: S::dummy(Dummy::D1),
        }
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{} -> ", self.id)?;

        let mut succ_iter = self.succ.iter();
        if let Some(succ) = succ_iter.next() {
            write!(f, "#{}", succ)?;

            while let Some(succ) = succ_iter.next() {
                write!(f, ", #{}", succ)?;
            }
        }

        write!(f, "  ::  defs: ")?;

        let mut defs_iter = self.defs.iter();
        if let Some(def_value) = defs_iter.next() {
            write!(f, "{:?}", def_value)?;

            while let Some(def_value) = defs_iter.next() {
                write!(f, ", {:?}", def_value)?;
            }
        }

        write!(f, " / uses: ")?;

        let mut uses_iter = self.uses.iter();
        if let Some(use_value) = uses_iter.next() {
            write!(f, "{:?}", use_value)?;

            while let Some(use_value) = uses_iter.next() {
                write!(f, ", {:?}", use_value)?;
            }
        }

        write!(f, " [{:?}]", self.loc)?;
        write!(f, "{:?}", self.metadata)?;

        Ok(())
    }
}

impl PartialEq for Statement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Statement {}

impl Hash for Statement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for Statement {
    fn partial_cmp(&self, other: &Statement) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Statement {
    fn cmp(&self, other: &Statement) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl DummyValue for Statement {
    fn dummy(dummy: Dummy) -> Self {
        Statement {
            id: StmtId::dummy(dummy),
            succ: Vec::new(),
            defs: Vec::new(),
            uses: Vec::new(),
            loc: Loc::dummy(dummy),
            metadata: Metadata::dummy(dummy),
            func: S::dummy(dummy),
        }
    }
}

impl_arena_type!(P<Statement>, Arena<Statement>);

impl P<Statement> {
    pub fn as_ref(&self) -> &Statement {
        Self::arena().get(self)
    }
}
