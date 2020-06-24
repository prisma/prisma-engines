mod capabilities;
mod tags;

pub use capabilities::*;
pub use tags::*;

use once_cell::sync::Lazy;

fn connector_names() -> Vec<(&'static str, Tags)> {
    vec![
        ("mysql_8", Tags::MYSQL | Tags::MYSQL_8),
        ("mysql", Tags::MYSQL),
        ("mysql_5_6", Tags::MYSQL | Tags::MYSQL_5_6),
        ("postgres9", Tags::POSTGRES),
        ("postgres", Tags::POSTGRES),
        ("postgres11", Tags::POSTGRES),
        ("postgres12", Tags::POSTGRES),
        ("mysql_mariadb", Tags::MYSQL | Tags::MARIADB),
        ("sqlite", Tags::SQLITE),
    ]
}

fn postgres_capabilities() -> Capabilities {
    Capabilities::SCALAR_LISTS | Capabilities::ENUMS | Capabilities::JSON
}

fn mysql_capabilities() -> Capabilities {
    Capabilities::ENUMS | Capabilities::JSON
}

fn mysql_5_6_capabilities() -> Capabilities {
    Capabilities::ENUMS
}

fn infer_capabilities(tags: Tags) -> Capabilities {
    if tags.intersects(Tags::POSTGRES) {
        return postgres_capabilities();
    }

    if tags.intersects(Tags::MYSQL_5_6) {
        return mysql_5_6_capabilities();
    }

    if tags.intersects(Tags::MYSQL) {
        return mysql_capabilities();
    }

    Capabilities::empty()
}

pub static CONNECTORS: Lazy<Connectors> = Lazy::new(|| {
    let connectors: Vec<Connector> = connector_names()
        .iter()
        .map(|(name, tags)| Connector {
            name: (*name).to_owned(),
            test_api_factory_name: format!("{}_test_api", name),
            capabilities: infer_capabilities(*tags),
            tags: *tags,
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
    pub capabilities: Capabilities,
    pub tags: Tags,
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
