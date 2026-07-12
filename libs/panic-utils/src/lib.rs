use std::{any::Any, borrow::Cow};

/// Downcasts a boxed [`Any`] from a panic payload to a string.
pub fn downcast_box_to_string(object: Box<dyn Any>) -> Option<Cow<'static, str>> {
    object
        .downcast::<&'static str>()
        .map(|s| Cow::Borrowed(*s))
        .or_else(|object| object.downcast::<String>().map(|s| Cow::Owned(*s)))
        .ok()
}

/// Downcasts a reference to [`Any`] from panic info hook to a string.
pub fn downcast_ref_to_string(object: &dyn Any) -> Option<&str> {
    object
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| object.downcast_ref::<String>().map(|s| s.as_str()))
}
