mod enumerator;

use crate::calculate_datamodel::InputContext;
pub(crate) use enumerator::EnumPair;

/// Holds the introspected item from the database, and a possible
/// previous value from the PSL.
#[derive(Clone, Copy)]
pub(crate) struct Pair<'a, T, U>
where
    T: Copy,
    U: Copy,
{
    previous: Option<T>,
    next: U,
    context: InputContext<'a>,
}

impl<'a, T, U> Pair<'a, T, U>
where
    T: Copy,
    U: Copy,
{
    pub(crate) fn new(context: InputContext<'a>, previous: Option<T>, next: U) -> Self {
        Self {
            context,
            previous,
            next,
        }
    }
}
