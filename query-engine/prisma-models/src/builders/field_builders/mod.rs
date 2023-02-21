mod composite_field_builder;
mod scalar_field_builder;

pub use composite_field_builder::*;
pub use scalar_field_builder::*;

use crate::{parent_container::ParentContainer, Field};

#[derive(Debug)]
pub enum FieldBuilder {
    Scalar(ScalarFieldBuilder),
}

impl FieldBuilder {
    pub fn build(self, container: ParentContainer) -> Field {
        match self {
            FieldBuilder::Scalar(st) => Field::Scalar(st.build(container)),
        }
    }
}
