//! Loading raw data produced by Aardwolf frontends and execution of
//! instrumented program.
//!
//! There are three types of data files:
//!
//! * *Static analysis data* -- Data produced by the frontend performing the
//!   static analysis of the program. It contains mostly the program structure
//!   and statement metadata.
//! * *Runtime data* -- Data produced by running the test suite with
//!   instrumented program. It is a long sequence of traced items, forming the
//!   execution trace and variable trace.
//! * *Test results* -- The results of running the test suite determining which
//!   test cases passed and which failed.
//!
//! The format of the first two is binary, while it is textual in test results
//! case. **The binary formats are not stable yet and will likely change!
//! Although we implement a version schema, the breaking changes will not be
//! indicated in the first version.**
//!
//! ## Static Analysis Data Format
//!
//! It starts with a magic sequence `0x41 0x41 0x52 0x44 0x2f 0x53` (i.e.,
//! `AARD/S` in ASCII) followed by the version number in ASCII (currently *1*,
//! i.e., `0x31`).
//!
//! The file is then sequence of byte tokens followed by the data specific for
//! the item the token represents. We now describe individual "data types" as
//! they are serialized into the binary form.
//!
//! **Statement-related types:**
//!
//! * `Statement`: `0xff ; GlobalId ; 1B for n_succ ; n_succ * GlobalId ; 1B for
//!   n_defs ; n_defs * Access ; 1B for n_uses ; n_uses * Access ; Loc ;
//!   Metadata`. All details for a statement.
//! * `FileId`: `8B`. Identifier of file (unique across the entire program, use
//!   filesystem capabilities).
//! * `StmtId`: `8B`. Identifier of statement in file (unique in the file the
//!   statement comes from).
//! * `GlobalId`: `FileId ; StmtId`. Global identifier of the statement which is
//!   unique across the entire program.
//! * `VarId`: `8B`. An identifier of a variable or a field of a structure.
//! * `Access`: `0xe0 ; VarId` or `0xe1 ; Access ; Access` or `0xe2 ; Access ;
//!   4B for n_index ; n_index * Access`. Access tree to a variable (supports
//!   scalar variables and structural and array accesses).
//! * `Loc`: `FileId ; 4B for line_begin ; 4B for col_begin ; 4B for line_end ;
//!   4B for col_end`. Statement location.
//! * `Metadata`: `1B`. Statement metadata.
//!
//! **Function-related types:**
//!
//! * `Function`: `0xfe ; null-terminated string`. Function name. All statements
//!   which follow this token are considered to belong to this function.
//!
//! **Other types:**
//!
//! * `Filenames`: `0xfd ; 4B for n_files ; n_files * Filename`. Mapping from
//!   numeric identifiers to actual file paths. The paths **must** be absolute.
//! * `Filename`: `FileId ; null-terminated string`.
//!
//! ## Runtime Data Format
//!
//! It starts with a magic sequence `0x41 0x41 0x52 0x44 0x2f 0x44` (i.e.,
//! `AARD/D` in ASCII) followed by the version number in ASCII (currently *1*,
//! i.e., `0x31`).
//!
//! The file is then sequence of byte tokens followed by the data specific for
//! the item the token represents. We now describe individual "data types" as
//! they are serialized into the binary form.
//!
//! **Execution trace types:**
//!
//! * `Statement`: `0xff ; GlobalId`. Indicates execution of statement
//!   identified by its global identifier.
//! * `External`: `0xfe ; null-terminated string`. Determines the start of a
//!   test case.
//!
//! **Variable trace types:**
//!
//! Data are identified by the token representing its data type and raw data
//! dumped after the token. Currently, for a valid variable trace we require a
//! value for every definition of the statement which precedes it. Using this
//! assumption, we do not need to indicate to which variable a value corresponds
//! as we can assign it using statement structure information.
//!
//! * `Unsupported`: `0x10`. A placeholder for variables whose data type is not
//!   supported by Aardwolf.
//! * `i8`: `0x11 ; 1B`.
//! * `i16`: `0x12 ; 2B`.
//! * `i32`: `0x13 ; 4B`.
//! * `i64`: `0x14 ; 8B`.
//! * `u8`: `0x15 ; 1B`.
//! * `u16`: `0x16 ; 2B`.
//! * `u32`: `0x17 ; 4B`.
//! * `u64`: `0x18 ; 8B`.
//! * `f32`: `0x19 ; 4B`.
//! * `f64`: `0x20 ; 8B`.
//! * `bool`: `0x21 ; 1B`. Any non-zero value is considered as *true*.
//!
//! ## Test results data format
//!
//! Test results are in textual form, where each test case name is on its line
//! prefixed either with `PASS: ` or `FAIL: `.

pub mod access;
mod consts;
pub mod module;
mod parser;
pub mod statement;
pub mod tests;
pub mod trace;
pub mod types;
pub mod values;

use std::collections::HashMap;
use std::hash::Hash;
use std::io::BufRead;

use crate::arena::{Arena, DummyValue, StringArena, P, S};

use module::Modules;
use tests::TestSuite;
use trace::Trace;
use types::FileId;
use values::{ValueArena, ValueRef};

pub use parser::{ParseError, ParseResult};

pub struct RawData {
    pub modules: Modules,
    pub trace: Trace,
    pub test_suite: TestSuite,
}

impl RawData {
    pub fn new(modules: Modules, trace: Trace, test_suite: TestSuite) -> Self {
        RawData {
            modules,
            trace,
            test_suite,
        }
    }

    pub fn parse<'a, R1: BufRead + 'a, R2: BufRead, R3: BufRead>(
        module_files: impl Iterator<Item = &'a mut R1>,
        trace_file: &mut R2,
        test_suite_file: &mut R3,
        ignore_corrupted: bool,
    ) -> parser::ParseResult<RawData> {
        let mut modules = Modules::new();
        let mut trace = Trace::new();
        let mut test_suite = TestSuite::new();
        let mut arenas = Arenas::new();

        for module_file in module_files {
            parser::parse_module(module_file, &mut modules, &mut arenas)?;
        }

        parser::parse_trace(trace_file, &mut trace, &mut arenas, ignore_corrupted)?;
        parser::parse_test_suite(test_suite_file, &mut test_suite, &mut arenas)?;

        // Set global singletons.
        arenas.seal();

        Ok(RawData::new(modules, trace, test_suite))
    }
}

struct UniqueArena<T> {
    arena: Arena<T>,
    cache: HashMap<Vec<u8>, P<T>>,
}

impl<T> UniqueArena<T> {
    fn alloc(&mut self, value: T, byte_repr: &[u8]) -> P<T> {
        // Double HashMap access, but key (vec of bytes) allocation only when
        // needed.
        if self.cache.contains_key(byte_repr) {
            *self.cache.get(byte_repr).unwrap()
        } else {
            let ptr = self.arena.alloc(value);
            self.cache.insert(byte_repr.to_owned(), ptr);
            ptr
        }
    }

    fn into_inner(self) -> Arena<T> {
        self.arena
    }
}

impl<T: DummyValue> UniqueArena<T> {
    fn with_capacity(capacity: usize) -> Self {
        UniqueArena {
            arena: Arena::with_capacity(capacity),
            cache: HashMap::with_capacity(capacity),
        }
    }
}

struct UniqueStringArena<T> {
    arena: StringArena<T>,
    cache: HashMap<String, S<T>>,
}

impl<T> UniqueStringArena<T> {
    fn with_capacity(capacity: usize) -> Self {
        UniqueStringArena {
            arena: StringArena::with_capacity(capacity),
            cache: HashMap::with_capacity(capacity),
        }
    }

    fn alloc<U: AsRef<str>>(&mut self, value: U) -> S<T> {
        // Double HashMap access, but key (String) allocation only when
        // needed.
        let value_ref = value.as_ref();

        if self.cache.contains_key(value_ref) {
            *self.cache.get(value_ref).unwrap()
        } else {
            let owned = value_ref.to_owned();
            let ptr = self.arena.alloc(&owned);
            self.cache.insert(owned, ptr);
            ptr
        }
    }

    fn into_inner(self) -> StringArena<T> {
        self.arena
    }
}

struct IdMap<T>(HashMap<T, usize>);

impl<T> IdMap<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        IdMap(HashMap::with_capacity(capacity))
    }
}

impl<T: Hash + Eq> IdMap<T> {
    pub fn get(&mut self, value: T) -> usize {
        let new_id = self.0.len();
        *self.0.entry(value).or_insert(new_id)
    }
}

pub(crate) struct Arenas {
    stmt_id: IdMap<(FileId, u64)>,
    stmt: UniqueArena<statement::Statement>,
    access: UniqueArena<access::Access>,
    value: ValueArena,
    func: UniqueStringArena<types::FuncName>,
    test: UniqueStringArena<types::TestName>,
    file: UniqueStringArena<types::FileName>,
}

impl Arenas {
    fn new() -> Self {
        Arenas {
            stmt_id: IdMap::with_capacity(1 << 16),
            stmt: UniqueArena::with_capacity(1 << 16),
            access: UniqueArena::with_capacity(1 << 16),
            value: ValueArena::with_capacity(1 << 16),
            func: UniqueStringArena::with_capacity(1 << 16),
            test: UniqueStringArena::with_capacity(1 << 16),
            file: UniqueStringArena::with_capacity(1 << 8),
        }
    }

    fn seal(self) {
        P::<statement::Statement>::init_once(self.stmt.into_inner());
        P::<access::Access>::init_once(self.access.into_inner());
        ValueRef::init_once(self.value);
        S::<types::FuncName>::init_once(self.func.into_inner());
        S::<types::TestName>::init_once(self.test.into_inner());
        S::<types::FileName>::init_once(self.file.into_inner());
    }
}
