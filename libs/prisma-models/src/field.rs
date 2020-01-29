mod relation;
mod scalar;

pub use relation::*;
pub use scalar::*;

use crate::prelude::*;
use once_cell::sync::OnceCell;
use std::{borrow::Cow, sync::Arc};

#[derive(Debug)]
pub enum FieldTemplate {
    Relation(RelationFieldTemplate),
    Scalar(ScalarFieldTemplate),
}

#[derive(Debug)]
pub enum Field {
    Relation(RelationFieldRef),
    Scalar(ScalarFieldRef),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FieldManifestation {
    pub db_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeIdentifier {
    String,
    Float,
    Boolean,
    Enum,
    Json,
    DateTime,
    GraphQLID,
    UUID,
    Int,
    Relation,
}

impl TypeIdentifier {
    pub fn user_friendly_type_name(self) -> String {
        match self {
            TypeIdentifier::GraphQLID => "ID".to_string(),
            _ => format!("{:?}", self),
        }
    }
}

impl Field {
    pub fn db_name(&self) -> Cow<str> {
        match self {
            Field::Scalar(ref sf) => Cow::from(sf.db_name()),
            Field::Relation(ref rf) => Cow::from(rf.db_name()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Field::Scalar(ref sf) => &sf.name,
            Field::Relation(ref rf) => &rf.name,
        }
    }

    pub fn is_scalar(&self) -> bool {
        match self {
            Field::Scalar(_) => true,
            Field::Relation(_) => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_list,
            Field::Relation(ref rf) => rf.is_list,
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Field::Scalar(ref sf) => sf.is_required,
            Field::Relation(ref rf) => rf.is_required,
        }
    }

    pub fn type_identifier(&self) -> TypeIdentifier {
        match self {
            Field::Scalar(ref sf) => sf.type_identifier,
            Field::Relation(ref rf) => rf.type_identifier,
        }
    }
}

impl FieldTemplate {
    pub fn build(self, model: ModelWeakRef) -> Field {
        match self {
            FieldTemplate::Scalar(st) => {
                let scalar = ScalarField {
                    name: st.name,
                    type_identifier: st.type_identifier,
                    is_required: st.is_required,
                    is_list: st.is_list,
                    is_id: st.is_id,
                    is_auto_generated_int_id: st.is_auto_generated_int_id,
                    is_unique: st.is_unique,
                    manifestation: st.manifestation,
                    internal_enum: st.internal_enum,
                    behaviour: st.behaviour,
                    model,
                    default_value: st.default_value,
                };

                Field::Scalar(Arc::new(scalar))
            }
            FieldTemplate::Relation(rt) => {
                let relation = RelationField {
                    name: rt.name,
                    type_identifier: rt.type_identifier,
                    is_required: rt.is_required,
                    is_list: rt.is_list,
                    is_auto_generated_int_id: rt.is_auto_generated_int_id,
                    is_unique: rt.is_unique,
                    relation_name: rt.relation_name,
                    relation_side: rt.relation_side,
                    model,
                    relation: OnceCell::new(),
                };

                Field::Relation(Arc::new(relation))
            }
        }
    }
}

impl From<ScalarFieldRef> for Field {
    fn from(sf: ScalarFieldRef) -> Self {
        Field::Scalar(sf)
    }
}

impl From<RelationFieldRef> for Field {
    fn from(rf: RelationFieldRef) -> Self {
        Field::Relation(rf)
    }
}
