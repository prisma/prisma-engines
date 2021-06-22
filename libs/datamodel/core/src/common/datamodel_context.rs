use crate::PreviewFeature;
use datamodel_connector::Connector;

//maybe we need a default source instead of a default connector? often the need for source properties goes hand in hand with the connector
pub struct DatamodelContext {
    // if active provider is none <-> connector is empty default
    pub source_name: Option<String>,
    pub active_provider: Option<String>,
    pub connector: Box<dyn Connector>,
    pub preview_features: Vec<PreviewFeature>,
}
