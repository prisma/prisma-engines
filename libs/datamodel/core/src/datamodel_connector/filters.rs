//! Client filter types.

#![deny(missing_docs)]

use enumflags2::*;

macro_rules! filters {
    ($(#[$docs:meta] $variant:ident,)*) => {
        /// Available filters for a given `String` scalar field.
        #[bitflags]
        #[derive(Debug, Clone, Copy)]
        #[repr(u8)]
        pub enum StringFilter {
            $(#[$docs] $variant),*
        }

        impl StringFilter {
            /// The property name of the filter in the client API.
            pub fn name(&self) -> String {
                let pascal_cased_name = match self {
                    $(StringFilter::$variant => stringify!($variant)),*
                };
                let mut out = pascal_cased_name.to_owned();
                out[0..1].make_ascii_lowercase(); // camel case it
                out
            }
        }
    };
}

filters! {
    /// String contains another string.
    Contains,
    /// String starts with another string.
    StartsWith,
    /// String ends with another string.
    EndsWith,
}
