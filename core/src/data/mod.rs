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
use std::io::BufRead;

use crate::arena::{Arena, DummyValue, StringArena, P, S};

use module::Modules;
use tests::TestSuite;
use trace::Trace;

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
    ) -> parser::ParseResult<RawData> {
        let mut modules = Modules::new();
        let mut trace = Trace::new();
        let mut test_suite = TestSuite::new();
        let mut arenas = Arenas::new();

        for module_file in module_files {
            parser::parse_module(module_file, &mut modules, &mut arenas)?;
        }

        parser::parse_trace(trace_file, &mut trace, &mut arenas)?;
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

pub(crate) struct Arenas {
    stmt: UniqueArena<statement::Statement>,
    access: UniqueArena<access::Access>,
    value: UniqueArena<values::Value>,
    func: UniqueStringArena<types::FuncName>,
    test: UniqueStringArena<types::TestName>,
    file: UniqueStringArena<types::FileName>,
}

impl Arenas {
    fn new() -> Self {
        Arenas {
            stmt: UniqueArena::with_capacity(1 << 16),
            access: UniqueArena::with_capacity(1 << 16),
            value: UniqueArena::with_capacity(1 << 16),
            func: UniqueStringArena::with_capacity(1 << 16),
            test: UniqueStringArena::with_capacity(1 << 16),
            file: UniqueStringArena::with_capacity(1 << 8),
        }
    }

    fn seal(self) {
        P::<statement::Statement>::init_once(self.stmt.into_inner());
        P::<access::Access>::init_once(self.access.into_inner());
        P::<values::Value>::init_once(self.value.into_inner());
        S::<types::FuncName>::init_once(self.func.into_inner());
        S::<types::TestName>::init_once(self.test.into_inner());
        S::<types::FileName>::init_once(self.file.into_inner());
    }
}
