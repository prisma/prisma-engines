//! Query schema builder. Root for query schema building.

mod enum_types;
mod input_types;
mod mutations;
mod output_types;
mod utils;

pub use self::{
    enum_types::itx_isolation_levels,
    utils::{compound_id_field_name, compound_index_field_name},
};

pub(crate) use output_types::{mutation_type, query_type};

use self::{enum_types::*, utils::*};
use crate::*;
use psl::{PreviewFeatures, datamodel_connector::ConnectorCapability};
use query_structure::{Field as ModelField, Model, RelationFieldRef, TypeIdentifier};

pub fn build(schema: Arc<psl::ValidatedSchema>, enable_raw_queries: bool) -> QuerySchema {
    let preview_features = schema.configuration.preview_features();

    build_with_features(schema, preview_features, enable_raw_queries)
}

pub fn build_with_features(
    schema: Arc<psl::ValidatedSchema>,
    preview_features: PreviewFeatures,
    enable_raw_queries: bool,
) -> QuerySchema {
    let connector = schema.connector;
    let internal_data_model = query_structure::convert(schema);

    QuerySchema::new(enable_raw_queries, connector, preview_features, internal_data_model)
}
