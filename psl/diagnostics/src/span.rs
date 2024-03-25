#[cfg(feature = "full-spans")]
mod full;
#[cfg(not(feature = "full-spans"))]
mod noop;

#[cfg(feature = "full-spans")]
pub use full::{FileId, Span};
#[cfg(not(feature = "full-spans"))]
pub use noop::{FileId, Span};
