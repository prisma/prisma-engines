#![no_std]

#[macro_use]
extern crate alloc;

mod collection;
mod error;
mod native_type_error_factory;
mod pretty_print;
mod span;
mod warning;

pub use collection::Diagnostics;
pub use error::DatamodelError;
pub use native_type_error_factory::NativeTypeErrorFactory;
pub use span::{FileId, Span};
pub use warning::DatamodelWarning;
