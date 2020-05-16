use std::any::type_name;
use std::cmp::Ordering;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

// `P` struct is completely opaque to the user. The only thing they can do with
// it is to get its pointed value using the appropriate arena.
//
// In order to derived equality trait implementations to work properly, the
// allocator must ensure that equal values are given exactly the same `P`.
#[derive(PartialEq, Eq, Hash)]
pub struct P<T> {
    index: u32,
    // Specify the type of value which this pointer represents. If there is
    // unique arena for each type, getting the value using `P` is safe and
    // guaranteed to success.
    typ: PhantomData<T>,
}

impl<T> Clone for P<T> {
    fn clone(&self) -> Self {
        P {
            index: self.index,
            typ: PhantomData,
        }
    }
}

impl<T> Copy for P<T> {}

impl<T> fmt::Debug for P<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("P")
            .field("index", &self.index)
            .field("type", &type_name::<T>())
            .finish()
    }
}

pub trait DummyValue {
    fn dummy(dummy: Dummy) -> Self;
}

#[derive(Clone, Copy)]
pub enum Dummy {
    D1,
    D2,
    D3,
    D4,
    D5,
}

impl Dummy {
    fn from_num(num: usize) -> Self {
        debug_assert!(num < Dummy::count());

        match num {
            0 => Dummy::D1,
            1 => Dummy::D2,
            2 => Dummy::D3,
            3 => Dummy::D4,
            4 => Dummy::D5,
            _ => unreachable!(),
        }
    }

    pub fn as_num(&self) -> usize {
        *self as usize
    }

    pub fn count() -> usize {
        5
    }
}

pub struct Arena<T> {
    storage: Vec<T>,
}

// It is important that constructors and `alloc` are visible only in the crate
// so external libraries cannot create new arenas and allocate in them. Even
// when Arena is not lifetime-restricted, if it is dropped, then its uniqueness
// guarantees that the living `P`s are useless and cannot be used to getting
// value references.
impl<T> Arena<T> {
    pub(crate) const fn empty() -> Self {
        Arena {
            storage: Vec::new(),
        }
    }

    pub(crate) fn alloc(&mut self, value: T) -> P<T> {
        assert!(
            self.storage.len() <= u32::MAX as usize,
            "maximum number of values exceeded"
        );
        let ptr = P {
            index: self.storage.len() as u32,
            typ: PhantomData,
        };

        self.storage.push(value);

        ptr
    }

    pub fn get(&self, ptr: &P<T>) -> &T {
        &self.storage[ptr.index as usize]
    }

    pub fn get_mut(&mut self, ptr: &P<T>) -> &mut T {
        &mut self.storage[ptr.index as usize]
    }
}

impl<T: DummyValue> Arena<T> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self::with_storage(Vec::with_capacity(capacity))
    }

    fn with_storage(storage: Vec<T>) -> Self {
        let mut this = Arena { storage };

        for idx in 0..Dummy::count() {
            this.alloc(T::dummy(Dummy::from_num(idx)));
        }

        this
    }

    pub fn dummy(dummy: Dummy) -> P<T> {
        P {
            index: dummy.as_num() as u32,
            typ: PhantomData,
        }
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct S<T> {
    // +--------------------+------------+
    // |     index (20)     |  len (12)  |
    // +--------------------+------------+
    repr: u32,
    typ: PhantomData<T>,
}

// 2^20 = 1_048_576 of strings
const INDEX_BITWIDTH: u32 = 20;
// 2^12 = 4096 maximum length of each string
const LEN_BITWIDTH: u32 = 12;

const MASK_INDEX: u32 = !0 << LEN_BITWIDTH;
const MASK_LEN: u32 = !0 >> INDEX_BITWIDTH;

impl<T> S<T> {
    fn new(index: usize, size: usize) -> Self {
        assert!(
            index < (1 << INDEX_BITWIDTH) as usize,
            "maximum number of strings exceeded"
        );
        assert!(
            size < (1 << LEN_BITWIDTH) as usize,
            "maximum size of string exceeded"
        );

        S {
            repr: ((index as u32) << LEN_BITWIDTH) | (size as u32),
            typ: PhantomData,
        }
    }

    fn index(&self) -> usize {
        ((self.repr & MASK_INDEX) >> LEN_BITWIDTH) as usize
    }

    fn len(&self) -> usize {
        (self.repr & MASK_LEN) as usize
    }
}

impl<T> Clone for S<T> {
    fn clone(&self) -> Self {
        S {
            repr: self.repr,
            typ: PhantomData,
        }
    }
}

impl<T> Copy for S<T> {}

impl<T> fmt::Debug for S<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("I")
            .field("index", &self.index())
            .field("len", &self.len())
            .field("type", &type_name::<T>())
            .finish()
    }
}

impl<T> DummyValue for S<T> {
    fn dummy(dummy: Dummy) -> Self {
        S::new(dummy.as_num(), 0)
    }
}

pub struct StringArena<T> {
    storage: String,
    typ: PhantomData<T>,
}

impl<T> StringArena<T> {
    pub(crate) const fn empty() -> Self {
        StringArena {
            storage: String::new(),
            typ: PhantomData,
        }
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        StringArena {
            storage: String::with_capacity(capacity),
            typ: PhantomData,
        }
    }

    pub(crate) fn alloc<U: AsRef<str>>(&mut self, value: U) -> S<T> {
        let value = value.as_ref();
        let index = self.storage.len();
        let ptr = S::new(index, value.len());

        self.storage.push_str(value);

        ptr
    }

    pub fn get(&self, ptr: &S<T>) -> &str {
        let lo = ptr.index();
        let hi = lo + ptr.len();
        &self.storage[lo..hi]
    }
}

// Cheap index-based Ord-related trait implementation for P pointers. The new
// type pattern is used not to confuse users by implementing this opaque
// ordering for P itself, since it does not relate to ordering of inner values
// at all.
#[derive(Clone, Copy, Hash, Debug)]
pub struct CheapOrd<T>(T);

impl<T> CheapOrd<T> {
    pub fn new(value: T) -> Self {
        CheapOrd(value)
    }
}

impl<T> PartialOrd for CheapOrd<P<T>> {
    fn partial_cmp(&self, other: &CheapOrd<P<T>>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for CheapOrd<P<T>> {
    fn cmp(&self, other: &CheapOrd<P<T>>) -> Ordering {
        self.0.index.cmp(&other.0.index)
    }
}

impl<T> PartialEq for CheapOrd<P<T>> {
    fn eq(&self, other: &CheapOrd<P<T>>) -> bool {
        self.0.index == other.0.index
    }
}

impl<T> Eq for CheapOrd<P<T>> {}

impl<T> Deref for CheapOrd<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Ideally custom derive macros.
macro_rules! impl_arena_type {
    ($ptr_ty:ty, $arena_ty:ty) => {
        impl $ptr_ty {
            // This function is unsafe as we return a mutable borrow of static
            // variable. If the user intends to use it actually mutably, they
            // must use a thread synchronization primitive.
            unsafe fn __arena() -> &'static mut $arena_ty {
                static mut ARENA: $arena_ty = <$arena_ty>::empty();
                &mut ARENA
            }

            fn __once() -> &'static std::sync::Once {
                static ONCE: std::sync::Once = std::sync::Once::new();
                &ONCE
            }

            pub fn arena() -> &'static $arena_ty {
                if <$ptr_ty>::__once().is_completed() {
                    // SAFETY: We only use arena immutably. Moreover, it is
                    // checked using Once that it is already initialized and
                    // will not be ever modified.
                    unsafe { <$ptr_ty>::__arena() }
                } else {
                    // Invalid usage of the Arena singleton. The arena, after
                    // allocating all data, must be assigned, so that the
                    // references can be used to get the actual values using the
                    // singleton. Note that theoretically, the initialization
                    // could be called and fail, but we consider it very
                    // unlikely as it is only assigning a variable in our case.
                    panic!("Arena not yet initialized.");
                }
            }

            pub(crate) fn init_once(arena: $arena_ty) {
                <$ptr_ty>::__once().call_once(|| {
                    // SAFETY: This is safe since it is performed inside Once
                    // synchronization primitive which will block other threads
                    // if an initialization is already being started. Moreover,
                    // reading from the arena is constrained such that it is
                    // already initialized, so no shared reference is taken from
                    // the arena during (and before) this mutating
                    // initialization.
                    unsafe { *<$ptr_ty>::__arena() = arena };
                })
            }
        }
    };
}
