#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::{fmt, ops::Deref};

use once_cell::race::OnceBox;

/// An alternative to `std::sync::LazyLock` or `once_cell::sync::Lazy` that does
/// not require locking and works without a dependency on `std` or
/// `critical_section`.
///
/// Unlike those types, it requires the initializer function to be `Fn() -> T`
/// and not `FnOnce() -> T`, and has slightly different semantics. If multiple
/// threads try to obtain the value before a `LazyRace` is initialized, all of
/// them will compute it, but only the first one to finish will store the value.
/// Other results will be discarded and replaced with the result produced by the
/// winning thread. Once a value has been stored, it will not be recomputed
/// anymore, and all subsequent concurrent accesses will return the cached
/// value.
pub struct LazyRace<T, F = fn() -> T> {
    cell: OnceBox<T>,
    init: F,
}

impl<T: fmt::Debug, F> fmt::Debug for LazyRace<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LazyRace")
            .field("cell", &self.cell)
            .field("init", &"..")
            .finish()
    }
}

impl<T, F> LazyRace<T, F> {
    pub const fn new(f: F) -> LazyRace<T, F> {
        LazyRace {
            cell: OnceBox::new(),
            init: f,
        }
    }
}

impl<T, F: Fn() -> T> LazyRace<T, F> {
    pub fn force(this: &Self) -> &T {
        this.cell.get_or_init(|| Box::new((this.init)()))
    }
}

impl<T, F: Fn() -> T> Deref for LazyRace<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        LazyRace::force(self)
    }
}
