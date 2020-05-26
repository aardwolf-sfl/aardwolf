//! Arena allocation.

use std::any::type_name;
use std::cmp::Ordering;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

/// Reference type for [`Arena`].
///
/// It is implemented as 4-byte numeric index (which limits the maximum number
/// of values an arena can hold). The generic type is used for distinguishing
/// between pointers belonging to different arenas.
///
/// The structure is completely opaque for the user and the only thing they can
/// do with it is to obtain the value which it points at in the corresponding
/// arena.
///
/// It implements equality traits by comparing the numeric index and **the user
/// must guarantee that equal values have identical `P` pointers** throughout
/// the whole program.
///
/// [`Arena`] struct.Arena.html
#[derive(PartialEq, Eq, Hash)]
pub struct P<T> {
    index: u32,
    // Specify the type of value which this pointer represents. If there is
    // unique arena for each type, getting the value using `P` is safe and
    // guaranteed to succeed.
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

/// An arena can pre-allocate some dummy values for types which implement this
/// trait. These can be used as some placeholders for values which are not
/// actually loaded from raw files. (Example: artificial ENTRY and EXIT
/// statements in control flow graph.)
pub trait DummyValue {
    fn dummy(dummy: Dummy) -> Self;
}

// TODO: I don't like this enum-based approach, at least not in  its pure form.
/// A fixed number of dummy values.
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

/// A classic arena for generic types.
///
/// It stores the values inside a vector. No magic happens here. If your type
/// could possibly benefit from a special representation, consider
/// implementation of custom arena.
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

    /// Allocates the value by simply adding it to the internal vector.
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

    /// Gets shared reference to the value represented by given pointer.
    pub fn get(&self, ptr: &P<T>) -> &T {
        &self.storage[ptr.index as usize]
    }

    /// Gets mutable reference to the value represented by given pointer.
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

/// Reference type for [`StringArena`].
///
/// It is implemented as 4-byte numeric number. The generic type is used for
/// distinguishing between pointers belonging to different arenas. The internal
/// representation stores the index and length of the string. The limits are
/// currently: 1_048_576 of strings, 4096 maximum length of each string.
///
/// The structure is completely opaque for the user and the only thing they can
/// do with it is to obtain the value which it points at in the corresponding
/// arena.
///
/// It implements equality traits by comparing the numeric index and **the user
/// must guarantee that equal values have identical `P` pointers** throughout
/// the whole program.
///
/// [`StringArena`] struct.StringArena.html
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

    /// Parses the index value from the internal representation.
    fn index(&self) -> usize {
        ((self.repr & MASK_INDEX) >> LEN_BITWIDTH) as usize
    }

    /// Parses the string length from the internal representation.
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

/// An arena customized for strings.
///
/// It appends strings to a single big string. This reduces the number of string
/// allocations.
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

    /// Gets the immutable reference to the string represented by given pointer.
    pub fn get(&self, ptr: &S<T>) -> &str {
        let lo = ptr.index();
        let hi = lo + ptr.len();
        &self.storage[lo..hi]
    }
}

/// Wrapper for [`P`] pointers that implements a cheap index-based ordering.
///
/// It does not consider the value it points to so the ordering is generally
/// wrong in the sense of the inner type semantics. It is mostly used for
/// allowing storing them in ordering-based collections.
///
/// The new type pattern is used not to confuse users. If the ordering was
/// implemented for `P` itself, it might be misleading and lead to hidden bugs.
///
/// [`P`] struct.P.html
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

/// Implements safe assignment and usage of global arena which belongs to the
/// pointer type. Thanks to this, the maintainer of the arena can allocate its
/// data and then initialize the global variable which can be later accessed by
/// all its pointers to reference real data from it.
///
/// It implements two static methods for public use: `init_once(arena)`
/// assigning given arena to the global variable and `arena()` returning
/// immutable static reference to the initialized arena.
///
/// The arena should be initialized exactly once. If it is never assigned, then
/// getting its reference will panic, if it is assigned multiple times, then the
/// other initializations do not make any effect. As mentioned, it must be
/// accessed only after the initialization. To function properly, user must
/// ensure these assumptions.
///
/// # Examples
///
/// ```
/// impl_arena_type!(P<YourType>, Arena<YourType>);
///
/// impl P<YourType> {
///    pub fn get(&self) -> &YourType {
///        Self::arena().get(self)
///    }
/// }
///
/// fn load() {
///     let mut arena = Arena::new();
///
///     // Data loading code.
///
///     // After this moment, the arena will be globally accessible.
///     P::<YourType>::init_once(arena);
/// }
/// ```
///
/// # Panics
///
/// Accessing the arena using `arena()` method before initializing it with
/// `init_once(arena)` will result into a panic. User must ensure that this
/// invariant holds in the program.
///
/// # Safety
///
/// Initialization of the global variable is guarded by [`Once`] synchronization
/// primitive. This ensures thread safety when multiple thread would try to
/// initialize the arena.
///
/// Accessing the arena is then completely safe as one gets immutable static
/// reference to it.
///
/// [`Once`]: https://doc.rust-lang.org/std/sync/struct.Once.html
#[macro_export]
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
