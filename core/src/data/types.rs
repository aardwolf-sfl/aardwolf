use std::fmt;

use crate::arena::{Dummy, DummyValue, StringArena, S};

// String-like types (FuncName, TestName, FileName) act only as a distinguishing
// opaque type for `StringArena`s.

#[derive(PartialEq, Eq, Hash)]
pub struct FuncName(());

impl_arena_type!(S<FuncName>, StringArena<FuncName>);

impl S<FuncName> {
    pub fn as_ref(&self) -> &str {
        Self::arena().get(self)
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct TestName(());

impl_arena_type!(S<TestName>, StringArena<TestName>);

impl S<TestName> {
    pub fn as_ref(&self) -> &str {
        Self::arena().get(self)
    }
}

#[derive(PartialEq, Eq, Hash)]
pub struct FileName(());

impl_arena_type!(S<FileName>, StringArena<FileName>);

impl S<FileName> {
    pub fn as_ref(&self) -> &str {
        Self::arena().get(self)
    }
}

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

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default, Debug)]
pub struct StmtId((FileId, u64));

impl StmtId {
    pub(crate) const fn new(file_id: FileId, stmt_id: u64) -> Self {
        StmtId((file_id, stmt_id))
    }

    #[cfg(test)]
    pub const fn new_test(stmt_id: u64) -> Self {
        StmtId((FileId(0), stmt_id))
    }
}

impl fmt::Display for StmtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", (self.0).0, (self.0).1)
    }
}

impl DummyValue for StmtId {
    fn dummy(dummy: Dummy) -> Self {
        StmtId((FileId::dummy(dummy), u64::MAX - (dummy.as_num() as u64)))
    }
}
