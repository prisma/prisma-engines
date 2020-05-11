//! MongoDB query connector for Prisma.

#![warn(missing_docs)]
#![warn(missing_debug_implementations, rust_2018_idioms)]
#![doc(test(attr(deny(rust_2018_idioms, warnings))))]
#![doc(test(attr(allow(unused_extern_crates, unused_variables))))]

mod connection;
mod connector;

pub use connection::Connection;
pub use connector::Connector;
