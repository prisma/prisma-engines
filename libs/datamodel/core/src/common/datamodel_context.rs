use crate::PreviewFeature;
use datamodel_connector::Connector;

pub struct DatamodelContext {
    // if sourcename is none <-> connector is empty default
    pub source_name: Option<String>,
    pub connector: Box<dyn Connector>,
    pub preview_features: Vec<PreviewFeature>,
}
