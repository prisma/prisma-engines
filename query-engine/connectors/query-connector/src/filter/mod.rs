//! Filtering types to select records from the database
//!
//! The creation of the types should be done with
//! [ScalarCompare](/query-connector/trait.ScalarCompare.html) and
//! [RelationCompare](/query-connector/trait.RelationCompare.html).

mod list;
mod relation;
mod scalar;

use prisma_models::prelude::*;
use prisma_models::DataSourceField;
use std::fmt;

pub use list::*;
pub use relation::*;
pub use scalar::*;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Filter {
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Vec<Filter>),
    Scalar(ScalarFilter),
    ScalarList(ScalarListFilter),
    OneRelationIsNull(OneRelationIsNullFilter),
    Relation(RelationFilter),
    NodeSubscription,
    BoolFilter(bool),
    Empty,
}

impl Filter {
    pub fn and(filters: Vec<Filter>) -> Self {
        Filter::And(filters)
    }

    pub fn or(filters: Vec<Filter>) -> Self {
        Filter::Or(filters)
    }

    pub fn not(filters: Vec<Filter>) -> Self {
        Filter::Not(filters)
    }

    pub fn empty() -> Self {
        Filter::Empty
    }

    /// Returns the size of the topmost filter elements (does not recursively compute the size).
    pub fn size(&self) -> usize {
        match self {
            Self::And(v) => v.len(),
            Self::Or(v) => v.len(),
            Self::Not(v) => v.len(),
            Self::Empty => 0,
            _ => 1,
        }
    }
}

// WIP
impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ScalarFilter> for Filter {
    fn from(sf: ScalarFilter) -> Self {
        Filter::Scalar(sf)
    }
}

impl From<ScalarListFilter> for Filter {
    fn from(sf: ScalarListFilter) -> Self {
        Filter::ScalarList(sf)
    }
}

impl From<OneRelationIsNullFilter> for Filter {
    fn from(sf: OneRelationIsNullFilter) -> Self {
        Filter::OneRelationIsNull(sf)
    }
}

impl From<RelationFilter> for Filter {
    fn from(sf: RelationFilter) -> Self {
        Filter::Relation(sf)
    }
}

impl From<bool> for Filter {
    fn from(b: bool) -> Self {
        Filter::BoolFilter(b)
    }
}

/// Creates a test data model for the unit tests in this module.
pub fn test_data_model() -> InternalDataModelRef {
    let user_field_templates = vec![
        FieldTemplate::Scalar(ScalarFieldTemplate {
            name: "id".to_owned(),
            type_identifier: TypeIdentifier::GraphQLID,
            is_required: true,
            is_list: false,
            is_unique: false,
            is_auto_generated_int_id: false,
            behaviour: None,
            internal_enum: None,
            data_source_field: DataSourceField {
                name: "id".to_owned(),
                default_value: None,
            },
        }),
        FieldTemplate::Scalar(ScalarFieldTemplate {
            name: "name".to_owned(),
            type_identifier: TypeIdentifier::String,
            is_required: false,
            is_list: false,
            is_unique: false,
            is_auto_generated_int_id: false,
            behaviour: None,
            internal_enum: None,
            data_source_field: DataSourceField {
                name: "name".to_owned(),
                default_value: None,
            },
        }),
        FieldTemplate::Relation(RelationFieldTemplate {
            name: "sites".to_owned(),
            type_identifier: TypeIdentifier::String,
            is_required: false,
            is_list: false,
            is_unique: false,
            is_auto_generated_int_id: false,
            relation_name: "bar".to_owned(),
            relation_side: RelationSide::A,
            data_source_fields: vec![DataSourceField {
                name: "sites".to_owned(),
                default_value: None,
            }],
        }),
    ];

    let site_field_templates = vec![FieldTemplate::Scalar(ScalarFieldTemplate {
        name: "name".to_owned(),
        type_identifier: TypeIdentifier::String,
        is_required: false,
        is_list: false,
        is_unique: false,
        is_auto_generated_int_id: false,
        behaviour: None,
        internal_enum: None,
        data_source_field: DataSourceField {
            name: "name".to_owned(),
            default_value: None,
        },
    })];

    let model_templates = vec![
        ModelTemplate {
            name: "User".to_owned(),
            is_embedded: false,
            fields: user_field_templates,
            manifestation: None,
            id_field_names: vec![],
            indexes: vec![],
        },
        ModelTemplate {
            name: "Site".to_owned(),
            is_embedded: false,
            fields: site_field_templates,
            manifestation: None,
            id_field_names: vec![],
            indexes: vec![],
        },
    ];

    let project_template = InternalDataModelTemplate {
        models: model_templates,
        relations: vec![],
        enums: vec![],
        version: None,
    };

    project_template.build("some_db_name".to_owned())
}
