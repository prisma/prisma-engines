use once_cell::sync::Lazy;

const CONNECTOR_NAMES: &[&'static str] = &[
    "mysql_8",
    "mysql",
    "postgres9",
    "postgres",
    "postgres11",
    "postgres12",
    "mysql_mariadb",
    "sqlite",
];

pub static CONNECTORS: Lazy<Connectors> = Lazy::new(|| {
    let connectors = CONNECTOR_NAMES
        .iter()
        .map(|name| Connector {
            name: (*name).to_owned(),
            test_api_factory_name: format!("{}_test_api", name),
        })
        .collect();
    Connectors::new(connectors)
});

pub struct Connectors {
    connectors: Vec<Connector>,
}

impl Connectors {
    fn new(connectors: Vec<Connector>) -> Connectors {
        Connectors { connectors }
    }

    pub fn all(&self) -> impl Iterator<Item = &Connector> {
        self.connectors.iter()
    }

    pub fn len(&self) -> usize {
        self.connectors.len()
    }
}

/// Represents a connector to be tested.
pub struct Connector {
    name: String,
    test_api_factory_name: String,
}

impl Connector {
    /// The name of the connector.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of the API factory function for that connector.
    pub fn test_api(&self) -> &str {
        &self.test_api_factory_name
    }
}
