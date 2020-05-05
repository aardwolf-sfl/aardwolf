use std::any::type_name;
use std::fmt;
use std::marker::PhantomData;

// `P` struct is completely opaque to the user. The only thing they can do with
// it is to get its pointed value using the appropriate arena.
//
// In order to derived trait implementation to work properly, the allocator must
// ensure that equal values are given exactly the same `P`.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct P<T> {
    index: usize,
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
        let ptr = P {
            index: self.storage.len(),
            typ: PhantomData,
        };

        self.storage.push(value);

        ptr
    }

    pub fn get(&self, ptr: &P<T>) -> &T {
        &self.storage[ptr.index]
    }

    pub fn get_mut(&mut self, ptr: &P<T>) -> &mut T {
        &mut self.storage[ptr.index]
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
            index: dummy.as_num(),
            typ: PhantomData,
        }
    }
}

#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct S<T> {
    lo: usize,
    hi: usize,
    typ: PhantomData<T>,
}

impl<T> Clone for S<T> {
    fn clone(&self) -> Self {
        S {
            lo: self.lo,
            hi: self.hi,
            typ: PhantomData,
        }
    }
}

impl<T> Copy for S<T> {}

impl<T> fmt::Debug for S<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("I")
            .field("lo", &self.lo)
            .field("hi", &self.hi)
            .field("type", &type_name::<T>())
            .finish()
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
        let lo = self.storage.len();

        let ptr = S {
            lo,
            hi: lo + value.len(),
            typ: PhantomData,
        };

        self.storage.push_str(value);

        ptr
    }

    pub fn get(&self, ptr: &S<T>) -> &str {
        &self.storage[ptr.lo..ptr.hi]
    }
}

// Ideally custom derive macros.
macro_rules! __impl_arena_type {
    ($ptr_ty:ty, $arena_ty:ty, $ret_ty:ty) => {
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

            // This is not implemented as `impl AsRef` now because I am not sure
            // if it satisfies the purpose of "cheap reference-to-reference
            // conversion". Maybe it is fine to do so, maybe it should be
            // renamed to a less confusing name.
            pub fn as_ref(&self) -> $ret_ty {
                if <$ptr_ty>::__once().is_completed() {
                    // SAFETY: We only use arena immutably.
                    unsafe { <$ptr_ty>::__arena() }.get(self)
                } else {
                    // Invalid usage of the Arena singleton. The arena, after allocating
                    // all data, must be assigned, so that the references can be used to
                    // get the actual values using the singleton. Note that
                    // theoretically, the initialization could be called and fail, but
                    // we consider it very unlikely as it is only assigning a variable
                    // in our case.
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

macro_rules! impl_arena_p {
    ($val_ty:ty) => {
        __impl_arena_type!(
            $crate::arena::P<$val_ty>,
            $crate::arena::Arena<$val_ty>,
            &$val_ty
        );
    };
}

macro_rules! impl_arena_s {
    ($val_ty:ty) => {
        __impl_arena_type!(
            $crate::arena::S<$val_ty>,
            $crate::arena::StringArena<$val_ty>,
            &str
        );
    };
}
