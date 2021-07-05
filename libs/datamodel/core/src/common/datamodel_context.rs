use crate::PreviewFeature;
use datamodel_connector::Connector;
use enumflags2::BitFlags;

pub struct DatamodelContext {
    pub source: Option<SourceContext>,
    pub preview_features: BitFlags<PreviewFeature>,
}

pub struct SourceContext {
    pub source_name: String,
    pub active_provider: String,
    pub connector: Box<dyn Connector>,
}
