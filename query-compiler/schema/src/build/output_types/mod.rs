pub(crate) mod aggregation;
pub(crate) mod field;
pub(crate) mod mutation_type;
pub(crate) mod objects;
pub(crate) mod query_type;

pub(crate) type FieldFn = Box<dyn (for<'a> Fn(&'a QuerySchema) -> OutputField<'a>) + Send + Sync>;

use super::*;
