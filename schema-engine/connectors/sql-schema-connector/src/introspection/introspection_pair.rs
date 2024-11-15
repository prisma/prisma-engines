//! A combination of PSL and a database definition.
//!
//! These modules are to be used to determine things such as field
//! names, attributes and so on.

mod default;
mod enumerator;
mod id;
mod index;
mod index_field;
mod model;
mod relation_field;
mod scalar_field;
mod view;

pub(crate) use default::{DefaultKind, DefaultValuePair};
pub(crate) use enumerator::EnumPair;
pub(crate) use id::IdPair;
pub(crate) use index::IndexPair;
pub(crate) use index_field::{IndexFieldPair, IndexOps};
pub(crate) use model::ModelPair;
pub(crate) use relation_field::{RelationFieldDirection, RelationFieldPair};
pub(crate) use scalar_field::ScalarFieldPair;
pub(crate) use view::ViewPair;

use crate::introspection::datamodel_calculator::DatamodelCalculatorContext;

/// Holds the introspected item from the database, and a possible
/// previous value from the PSL.
///
/// Please see the different pair implementations in the module for
/// details.
#[derive(Clone, Copy)]
pub(crate) struct IntrospectionPair<'a, T, U>
where
    T: Copy,
    U: Copy,
{
    /// The previous state, taken from the PSL.
    previous: T,
    /// The next state, taken from the database.
    next: U,
    /// The configuration object of the introspection.
    context: &'a DatamodelCalculatorContext<'a>,
}

impl<'a, T, U> IntrospectionPair<'a, T, U>
where
    T: Copy,
    U: Copy,
{
    pub(crate) fn new(context: &'a DatamodelCalculatorContext<'a>, previous: T, next: U) -> Self {
        Self {
            context,
            previous,
            next,
        }
    }
}
