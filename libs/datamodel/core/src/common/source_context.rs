use datamodel_connector::Connector;

pub struct SourceContext {
    pub source_name: String,
    pub active_provider: String,
    pub connector: Box<dyn Connector>,
}
