mod composite_field_builder;
mod scalar_field_builder;

pub use composite_field_builder::*;
pub use scalar_field_builder::*;

use crate::{parent_container::ParentContainer, CompositeTypeRef, Field};

#[derive(Debug)]
pub enum FieldBuilder {
    Scalar(ScalarFieldBuilder),
    Composite(CompositeFieldBuilder),
}

impl FieldBuilder {
    pub fn build(self, container: ParentContainer, composite_types: &[CompositeTypeRef]) -> Field {
        match self {
            FieldBuilder::Scalar(st) => Field::Scalar(st.build(container)),
            FieldBuilder::Composite(ct) => Field::Composite(ct.build(container, composite_types)),
        }
    }
}
