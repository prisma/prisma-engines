mod composite;
mod relation;
mod scalar;

use self::{composite::CompositeFieldBuilder, relation::RelationFieldBuilder, scalar::ScalarFieldBuilder};
use crate::{CompositeTypeRef, Field, ModelWeakRef};

#[derive(Debug)]
pub enum FieldBuilder {
    Relation(RelationFieldBuilder),
    Scalar(ScalarFieldBuilder),
    Composite(CompositeFieldBuilder),
}

impl FieldBuilder {
    pub fn build(self, model: ModelWeakRef, composite_types: &[CompositeTypeRef]) -> Field {
        match self {
            FieldBuilder::Scalar(st) => Field::Scalar(st.build(model)),
            FieldBuilder::Relation(rt) => Field::Relation(rt.build(model)),
            FieldBuilder::Composite(ct) => Field::Composite(ct.build(model, composite_types)),
        }
    }
}
