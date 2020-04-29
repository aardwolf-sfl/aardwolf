use std::fmt;
use std::rc::Rc;

// String-like types (FuncName, TestName) encapsulate string which are cheap to
// clone. At the moment, we use reference counting, but might use techniques
// like string interning or similar.
macro_rules! impl_encapsulate_string {
    ($container:ident) => {
        #[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
        pub struct $container(Rc<String>);

        impl $container {
            pub(crate) fn new<T: Into<String>>(func_name: T) -> Self {
                $container(Rc::new(func_name.into()))
            }
        }

        impl fmt::Display for $container {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0.as_str())
            }
        }

        impl PartialEq<str> for $container {
            fn eq(&self, other: &str) -> bool {
                self.0.as_str() == other
            }
        }
    };
}

impl_encapsulate_string!(FuncName);
impl_encapsulate_string!(TestName);

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default, Debug)]
pub struct FileId(u64);

impl FileId {
    pub(crate) const fn new(file_id: u64) -> Self {
        FileId(file_id)
    }

    pub const fn dummy(file_id: u64) -> Self {
        FileId(file_id)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, Default, Debug)]
pub struct StmtId((FileId, u64));

impl StmtId {
    pub(crate) const fn new(file_id: FileId, stmt_id: u64) -> Self {
        StmtId((file_id, stmt_id))
    }

    pub const fn dummy(stmt_id: u64) -> Self {
        StmtId((FileId::dummy(0), stmt_id))
    }
}

impl fmt::Display for StmtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", (self.0).0, (self.0).1)
    }
}
