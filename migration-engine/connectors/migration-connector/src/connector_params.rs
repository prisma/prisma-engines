use enumflags2::BitFlags;
use psl::PreviewFeature;

/// Parameters passed from the core to connectors on initialization.
pub struct ConnectorParams {
    /// The raw connection string or `url` datasource property.
    pub connection_string: String,
    /// The opted-into preview features.
    pub preview_features: BitFlags<PreviewFeature>,
    /// The schemas to query for PreviewFeature::MultiSchema.
    pub namespaces: Vec<String>,
    /// The shadow database connection string.
    pub shadow_database_connection_string: Option<String>,
}
