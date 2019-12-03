//! Filtering types to select records from the database
//!
//! The creation of the types should be done with
//! [ScalarCompare](/query-connector/trait.ScalarCompare.html) and
//! [RelationCompare](/query-connector/trait.RelationCompare.html).

mod list;
mod record_finder;
mod relation;
mod scalar;

pub use list::*;
use prisma_models::prelude::*;
pub use record_finder::*;
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
        Filter::BoolFilter(true)
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

impl From<RecordFinder> for Filter {
    fn from(record_finder: RecordFinder) -> Self {
        Filter::Scalar(ScalarFilter {
            field: record_finder.field,
            condition: ScalarCondition::Equals(record_finder.value),
        })
    }
}

impl From<Option<RecordFinder>> for Filter {
    fn from(record_finder: Option<RecordFinder>) -> Self {
        match record_finder {
            Some(rf) => Self::from(rf),
            None => Self::empty(),
        }
    }
}

impl From<Vec<RecordFinder>> for Filter {
    fn from(record_finders: Vec<RecordFinder>) -> Self {
        if record_finders.is_empty() {
            Self::empty()
        } else {
            let as_filters: Vec<Filter> = record_finders.into_iter().map(|x| x.into()).collect();
            Filter::or(as_filters).into()
        }
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
            is_hidden: false,
            is_auto_generated: false,
            manifestation: None,
            behaviour: None,
            default_value: None,
            internal_enum: None,
        }),
        FieldTemplate::Scalar(ScalarFieldTemplate {
            name: "name".to_owned(),
            type_identifier: TypeIdentifier::String,
            is_required: false,
            is_list: false,
            is_unique: false,
            is_hidden: false,
            is_auto_generated: false,
            manifestation: None,
            behaviour: None,
            default_value: None,
            internal_enum: None,
        }),
        FieldTemplate::Relation(RelationFieldTemplate {
            name: "sites".to_owned(),
            type_identifier: TypeIdentifier::String,
            is_required: false,
            is_list: false,
            is_unique: false,
            is_hidden: false,
            is_auto_generated: false,
            manifestation: None,
            relation_name: "bar".to_owned(),
            relation_side: RelationSide::A,
        }),
    ];

    let site_field_templates = vec![FieldTemplate::Scalar(ScalarFieldTemplate {
        name: "name".to_owned(),
        type_identifier: TypeIdentifier::String,
        is_required: false,
        is_list: false,
        is_unique: false,
        is_hidden: false,
        is_auto_generated: false,
        manifestation: None,
        behaviour: None,
        default_value: None,
        internal_enum: None,
    })];

    let model_templates = vec![
        ModelTemplate {
            name: "User".to_owned(),
            is_embedded: false,
            fields: user_field_templates,
            manifestation: None,
            indexes: vec![],
        },
        ModelTemplate {
            name: "Site".to_owned(),
            is_embedded: false,
            fields: site_field_templates,
            manifestation: None,
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
