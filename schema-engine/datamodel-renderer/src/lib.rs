//! A set of datastructures meant for rendering a Prisma data model as
//! a string. We don't even try to make the result pretty. Please use
//! the functionality of prisma-fmt for that.
//!
//! All structs implement `std::fmt::Display` for easy usage.
//!
//! An example use case to render a datasource as a string:
//!
//! ```
//! use datamodel_renderer::{configuration::Datasource, value::Env};
//! use indoc::{indoc, formatdoc};
//!
//! let datasource = Datasource::new(
//!     "db",
//!     "postgres",
//! );
//!
//! // We get a string rendering without proper formatting
//! // by calling the `to_string()` method:
//! let rendered = datasource.to_string();
//!
//! // The output is not formatted, so we call the reformat
//! // function to the result to make it look more kosher.
//! let rendered = psl::reformat(&rendered, 2).unwrap();
//!
//! let expected = indoc! {r#"
//!     datasource db {
//!       provider = "postgres"
//!     }
//! "#};
//!
//! assert_eq!(expected, &rendered);
//!
//! // Additionally we can just pass the datasource to any
//! // format block to include it in the resulting string:
//! let rendered = formatdoc!(r#"
//!     {datasource}
//!
//!     model A {{
//!       id Int @id
//!     }}
//! "#);
//!
//! // Again, making the result indentation and spacing to
//! // look prettier.
//! let rendered = psl::reformat(&rendered, 2).unwrap();
//!
//! let expected = indoc! {r#"
//!     datasource db {
//!       provider = "postgres"
//!     }
//!
//!     model A {
//!       id Int @id
//!     }
//! "#};
//!
//! assert_eq!(expected, &rendered);
//! ```

#![warn(missing_docs)]

pub mod configuration;
pub mod datamodel;
pub mod value;

pub use configuration::Configuration;
pub use datamodel::Datamodel;

use std::borrow::Cow;
