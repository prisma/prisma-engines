mod tags;

pub use once_cell::sync::Lazy;
pub use tags::*;

pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    tokio::runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn connector_names() -> Vec<(&'static str, Tags)> {
    vec![
        ("mssql", Tags::MSSQL),
        ("mysql", Tags::MYSQL),
        ("postgres", Tags::POSTGRES),
        ("sqlite", Tags::SQLITE),
    ]
}

pub static CONNECTORS: Lazy<Connectors> = Lazy::new(|| {
    let connectors: Vec<ConnectorDefinition> = connector_names()
        .iter()
        .map(|(name, tags)| ConnectorDefinition {
            name: (*name).to_owned(),
            test_api_factory_name: format!("{}_test_api", name),
            tags: *tags,
        })
        .collect();

    Connectors::new(connectors)
});

pub struct Connectors {
    connectors: Vec<ConnectorDefinition>,
}

impl Connectors {
    fn new(connectors: Vec<ConnectorDefinition>) -> Connectors {
        Connectors { connectors }
    }

    pub fn all(&self) -> impl Iterator<Item = &ConnectorDefinition> {
        self.connectors.iter()
    }

    pub fn len(&self) -> usize {
        self.connectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Represents a connector to be tested.
pub struct ConnectorDefinition {
    name: String,
    test_api_factory_name: String,
    pub tags: Tags,
}

impl ConnectorDefinition {
    /// The name of the connector.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of the API factory function for that connector.
    pub fn test_api(&self) -> &str {
        &self.test_api_factory_name
    }
}
