//! This code is EOL and nothing new should be added here anymore.

mod lift;

pub mod default_value;
pub mod native_type_instance;

pub use self::{default_value::*, lift::dml_default_kind, native_type_instance::*};
pub use prisma_value::{self, PrismaValue};
pub use psl_core::parser_database::{ast::FieldArity, IndexType, ReferentialAction};
