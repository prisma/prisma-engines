use crate::PreviewFeature;
use datamodel_connector::Connector;

pub struct DatamodelContext {
    pub connector: Option<Box<dyn Connector>>,
    pub preview_features: Vec<PreviewFeature>,
}

impl DatamodelContext {
    pub fn connector_ref(&self) -> Option<&dyn Connector> {
        self.connector.as_ref().map(|c| c.as_ref())
    }
}
