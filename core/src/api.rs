//! API used throughout the Aardwolf ecosystem, mainly in fault localization
//! plugins.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

use crate::data::{types::FileId, RawData};
use crate::queries::{Query, QueryArgs, QueryKey};

#[derive(Debug)]
pub enum EmptyDataReason {
    Static,
    Runtime,
    TestSuite,
}

#[derive(Debug)]
pub enum InvalidData {
    NoFailingTest,
    Empty(EmptyDataReason),
}

impl fmt::Display for InvalidData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidData::NoFailingTest => write!(f, "no failing test case"),
            InvalidData::Empty(EmptyDataReason::Static) => {
                write!(f, "Aardwolf static data is empty")
            }
            InvalidData::Empty(EmptyDataReason::Runtime) => {
                write!(f, "Aardwolf runtime data is empty")
            }
            InvalidData::Empty(EmptyDataReason::TestSuite) => {
                write!(f, "Aardwolf test results data is empty")
            }
        }
    }
}

/// The main entry point for Aardwolf plugins.
///
/// It holds the raw data and offers the API for querying these data using
/// [`Query`] system. All the queries are memoized so they do not need to be
/// recomputed if requested again.
///
/// Moreover, it provides several useful methods which retrieve some information
/// from the raw data in a nice form.
///
/// [`Query`]: ../queries/index.html
pub struct Api {
    data: RawData,
    // RefCell enables to mutably borrow the cache even when Api is borrowed
    // immutably. Use of Rc allows us to safely return a reference to the cached
    // query without the need of expensively cloning it.
    queries: RefCell<HashMap<(QueryKey, TypeId), Rc<dyn Any>>>,
}

impl Api {
    pub(crate) fn new(data: RawData) -> Result<Self, InvalidData> {
        if data.modules.files.is_empty() || data.modules.functions.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Static))
        } else if data.trace.trace.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::Runtime))
        } else if data.test_suite.tests.is_empty() {
            Err(InvalidData::Empty(EmptyDataReason::TestSuite))
        } else if data
            .test_suite
            .tests
            .values()
            .all(|status| status.is_passed())
        {
            Err(InvalidData::NoFailingTest)
        } else {
            Ok(Api {
                data,
                queries: RefCell::new(HashMap::new()),
            })
        }
    }

    /// Makes the query without any argument. See [`query_with`] for more
    /// details.
    ///
    /// This function is not thread-safe!
    ///
    /// [`query_with`]: #method.query_with
    pub fn query<Q: Query<Args = ()>>(&self) -> Result<Rc<Q>, Q::Error> {
        self.query_with(&())
    }

    /// Makes the query with given argument. All queries are type-based and by
    /// their type and the argument they are also memoized. The return value is
    /// either a reference-counted pointer or an error if it happens.
    ///
    /// This function is not thread-safe!
    pub fn query_with<Q: Query>(&self, args: &Q::Args) -> Result<Rc<Q>, Q::Error> {
        let type_id = TypeId::of::<Q>();
        let key = args.key();

        // We cannot use `entry` API since it would break support for nested
        // queries as it would violate exclusive mutable borrow rules enforced
        // by RefCell.
        let value = if !self.queries.borrow().contains_key(&(key.clone(), type_id)) {
            // If a query, whose creation is erroneous, is requested multiple
            // times, it is also recomputed (with failed result) multiple times
            // since it is not stored in the cache. We accept this behavior
            // since we consider failed query to be an ill state for the
            // localization and such process would end up with an error early.
            //
            // It is important that calling `Q::init` is done *before* mutably
            // borrowing the queries cache. In this way, nested queries will
            // work fine because at this point, the cache is not borrowed by
            // anything.
            let value = Rc::new(Q::init(&self.data, args, self)?);

            // Create a new pointer to the now-compted query.
            self.queries
                .borrow_mut()
                .insert((key, type_id), value.clone());

            value
        } else {
            // The query is already in the cache. We can safely unwrap it and
            // create a new pointer to the cached query.
            let value = self.queries.borrow().get(&(key, type_id)).unwrap().clone();

            // Cast the value to the concrete type. Since we store the
            // any-values by their type id, we are sure that the cast will end
            // up successful.
            value.downcast::<Q>().unwrap()
        };

        Ok(value)
    }

    /// Gets relative file path corresponding to given file identifier. The path
    /// is relative to the current working directory, but this will change in
    /// the future.
    pub fn file(&self, file_id: &FileId) -> Option<PathBuf> {
        self.full_file(file_id)?
            // TODO: Should be project root directory, that is, the directory
            // where `.aardwolf.yml` is.
            .strip_prefix(env::current_dir().ok()?)
            .ok()
            .map(|path| path.to_path_buf())
    }

    /// Gets the absolute path corresponding to given file identifier as is
    /// stored in the raw data.
    pub fn full_file(&self, file_id: &FileId) -> Option<PathBuf> {
        let ptr = self.data.modules.files.get(file_id)?;
        let raw = PathBuf::from(ptr.as_ref());
        raw.canonicalize().ok().map(|path| path.to_path_buf())
    }
}
