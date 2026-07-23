use enumflags2::BitFlags;
use psl::PreviewFeature;

/// Parameters passed from the core to connectors on initialization.
#[derive(Debug, Clone)]
pub struct ConnectorParams {
    /// The raw connection string or `url` datasource property.
    pub connection_string: String,
    /// The opted-into preview features.
    pub preview_features: BitFlags<PreviewFeature>,
    /// The shadow database connection string.
    pub shadow_database_connection_string: Option<String>,
}

impl ConnectorParams {
    /// Creates new [`ConnectorParams`].
    pub fn new(
        connection_string: String,
        preview_features: BitFlags<PreviewFeature>,
        shadow_database_connection_string: Option<String>,
    ) -> Self {
        Self {
            connection_string,
            preview_features,
            shadow_database_connection_string,
        }
    }
}
