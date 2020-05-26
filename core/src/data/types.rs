//! Simple types used in other data structures.

use std::fmt;

use crate::arena::{Dummy, DummyValue, StringArena, S};

// String-like types (FuncName, TestName, FileName) act only as a distinguishing
// opaque type for `StringArena`s.

/// A placeholder type for function name (see [`Arena`] module for more
/// details).
///
/// [`Arena`]: ../../arena/index.html
#[derive(PartialEq, Eq, Hash)]
pub struct FuncName(());

impl_arena_type!(S<FuncName>, StringArena<FuncName>);

impl S<FuncName> {
    /// Gets the actual function name of the reference from the arena.
    pub fn as_ref(&self) -> &str {
        Self::arena().get(self)
    }
}

/// A placeholder type for test case name (see [`Arena`] module for more
/// details).
///
/// [`Arena`]: ../../arena/index.html
#[derive(PartialEq, Eq, Hash)]
pub struct TestName(());

impl_arena_type!(S<TestName>, StringArena<TestName>);

impl S<TestName> {
    /// Gets the actual test case name of the reference from the arena.
    pub fn as_ref(&self) -> &str {
        Self::arena().get(self)
    }
}

/// A placeholder type for file path (see [`Arena`] module for more details).
///
/// [`Arena`]: ../../arena/index.html
#[derive(PartialEq, Eq, Hash)]
pub struct FileName(());

impl_arena_type!(S<FileName>, StringArena<FileName>);

impl S<FileName> {
    /// Gets the actual file path of the reference from the arena.
    pub fn as_ref(&self) -> &str {
        Self::arena().get(self)
    }
}

/// Globally unique file identifier.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default, Debug)]
pub struct FileId(u64);

impl FileId {
    pub(crate) const fn new(file_id: u64) -> Self {
        FileId(file_id)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl DummyValue for FileId {
    fn dummy(dummy: Dummy) -> Self {
        FileId(u64::MAX - (dummy.as_num() as u64))
    }
}

/// Globally unique statement identifier.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default, Debug)]
pub struct StmtId(u32);

impl StmtId {
    pub(crate) fn new(stmt_id: usize) -> Self {
        assert!(
            stmt_id <= u32::MAX as usize,
            "maximum statement id exceeded"
        );
        StmtId(stmt_id as u32)
    }

    #[cfg(test)]
    pub const fn new_test(stmt_id: usize) -> Self {
        StmtId(stmt_id as u32)
    }
}

impl fmt::Display for StmtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl DummyValue for StmtId {
    fn dummy(dummy: Dummy) -> Self {
        StmtId(u32::MAX - (dummy.as_num() as u32))
    }
}
