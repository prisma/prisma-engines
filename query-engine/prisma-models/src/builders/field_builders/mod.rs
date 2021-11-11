mod composite_field_builder;
mod relation_field_builder;
mod scalar_field_builder;

pub use composite_field_builder::*;
pub use relation_field_builder::*;
pub use scalar_field_builder::*;

use crate::{parent_container::ParentContainer, CompositeTypeRef, Field};

#[derive(Debug)]
pub enum FieldBuilder {
    Relation(RelationFieldBuilder),
    Scalar(ScalarFieldBuilder),
    Composite(CompositeFieldBuilder),
}

impl FieldBuilder {
    pub fn build(self, container: ParentContainer, composite_types: &[CompositeTypeRef]) -> Field {
        match self {
            FieldBuilder::Scalar(st) => Field::Scalar(st.build(container)),
            FieldBuilder::Relation(rt) => Field::Relation(rt.build(container.as_model_weak().unwrap())), // Relations are only possible between models.
            FieldBuilder::Composite(ct) => Field::Composite(ct.build(container, composite_types)),
        }
    }
}
