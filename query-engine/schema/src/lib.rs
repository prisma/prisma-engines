#![warn(warnings)]
#![allow(clippy::derive_partial_eq_without_eq)]

mod enum_type;
mod input_types;
mod output_types;
mod query_schema;
mod renderer;

pub use enum_type::*;
pub use input_types::*;
pub use output_types::*;
pub use query_schema::*;
pub use renderer::*;

use std::sync::{Arc, Weak};

pub type ObjectTypeStrongRef = Arc<ObjectType>;
pub type ObjectTypeWeakRef = Weak<ObjectType>;

pub type InputObjectTypeStrongRef = Arc<InputObjectType>;
pub type InputObjectTypeWeakRef = Weak<InputObjectType>;

pub type QuerySchemaRef = Arc<QuerySchema>;
pub type OutputTypeRef = Arc<OutputType>;
pub type OutputFieldRef = Arc<OutputField>;
pub type InputFieldRef = Arc<InputField>;

pub type EnumTypeRef = Arc<EnumType>;
pub type EnumTypeWeakRef = Weak<EnumType>;

/// Since we have the invariant that the weak refs that are used throughout the query
/// schema have to be always valid, we use this simple trait to keep the code clutter low.
pub trait IntoArc<T> {
    #[allow(clippy::wrong_self_convention)]
    fn into_arc(&self) -> Arc<T>;
}

impl<T> IntoArc<T> for Weak<T> {
    fn into_arc(&self) -> Arc<T> {
        self.upgrade().expect("Expected weak reference to be valid.")
    }
}

#[derive(Debug, PartialEq)]
pub struct Deprecation {
    pub since_version: String,
    pub planned_removal_version: Option<String>,
    pub reason: String,
}
