#![no_std]

extern crate alloc;

mod fragment;
mod placeholder;
mod template;

pub use fragment::Fragment;
pub use placeholder::PlaceholderFormat;
pub use template::QueryTemplate;
