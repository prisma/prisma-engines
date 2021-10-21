mod composite_field_builder;
mod relation_field_builder;
mod scalar_field_builder;

pub use composite_field_builder::*;
pub use relation_field_builder::*;
pub use scalar_field_builder::*;

use crate::{CompositeTypeRef, Field, ModelWeakRef};

#[derive(Debug)]
pub enum FieldBuilder {
    Relation(RelationFieldBuilder),
    Scalar(ScalarFieldBuilder),
    Composite(CompositeFieldBuilder),
}

impl FieldBuilder {
    pub fn build(self, model: ModelWeakRef, _composite_types: &[CompositeTypeRef]) -> Field {
        match self {
            FieldBuilder::Scalar(st) => Field::Scalar(st.build(model)),
            FieldBuilder::Relation(rt) => Field::Relation(rt.build(model)),
            FieldBuilder::Composite(_ct) => todo!(), // Field::Composite(ct.build(model, composite_types)),
        }
    }
}
