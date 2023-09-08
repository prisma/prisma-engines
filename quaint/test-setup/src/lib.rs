mod tags;

pub use once_cell::sync::Lazy;
pub use tags::*;

pub fn run_with_tokio<O, F: std::future::Future<Output = O>>(fut: F) -> O {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(fut)
}

fn connector_names() -> Vec<(&'static str, &'static str, Tags)> {
    vec![
        ("mssql", "mssql", Tags::MSSQL),
        ("mysql5_7", "mysql", Tags::MYSQL5_7),
        ("mysql8", "mysql", Tags::MYSQL8),
        ("mysql_mariadb", "mysql", Tags::MYSQL_MARIADB),
        ("postgresql", "postgresql", Tags::POSTGRES),
        ("sqlite", "sqlite", Tags::SQLITE),
    ]
}

pub static CONNECTORS: Lazy<Connectors> = Lazy::new(|| {
    let connectors: Vec<ConnectorDefinition> = connector_names()
        .iter()
        .map(|(name, feature_name, tags)| ConnectorDefinition {
            name: (*name).to_owned(),
            feature_name: (*feature_name).to_owned(),
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
#[derive(Debug)]
pub struct ConnectorDefinition {
    name: String,
    feature_name: String,
    test_api_factory_name: String,
    pub tags: Tags,
}

impl ConnectorDefinition {
    /// The name of the connector.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The feature name of the connector.
    pub fn feature_name(&self) -> &str {
        &self.feature_name
    }

    /// The name of the API factory function for that connector.
    pub fn test_api(&self) -> &str {
        &self.test_api_factory_name
    }
}
