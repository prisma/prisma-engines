mod capabilities;
mod tags;

pub use capabilities::*;
pub use tags::*;

use enumflags2::BitFlags;
use once_cell::sync::Lazy;

fn connector_names() -> Vec<(&'static str, BitFlags<Tags>)> {
    vec![
        ("mysql_8", Tags::Mysql | Tags::Mysql8),
        ("mysql", Tags::Mysql.into()),
        ("mysql_5_6", Tags::Mysql | Tags::Mysql56),
        ("postgres9", Tags::Postgres.into()),
        ("postgres", Tags::Postgres.into()),
        ("postgres11", Tags::Postgres.into()),
        ("postgres12", Tags::Postgres.into()),
        ("postgres13", Tags::Postgres.into()),
        ("mysql_mariadb", Tags::Mysql | Tags::Mariadb),
        ("sqlite", Tags::Sqlite.into()),
    ]
}

fn postgres_capabilities() -> BitFlags<Capabilities> {
    Capabilities::ScalarLists | Capabilities::Enums | Capabilities::Json
}

fn mysql_capabilities() -> BitFlags<Capabilities> {
    Capabilities::Enums | Capabilities::Json
}

fn mysql_5_6_capabilities() -> BitFlags<Capabilities> {
    Capabilities::Enums.into()
}

fn infer_capabilities(tags: BitFlags<Tags>) -> BitFlags<Capabilities> {
    if tags.intersects(Tags::Postgres) {
        return postgres_capabilities();
    }

    if tags.intersects(Tags::Mysql56) {
        return mysql_5_6_capabilities();
    }

    if tags.intersects(Tags::Mysql) {
        return mysql_capabilities();
    }

    BitFlags::empty()
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Represents a connector to be tested.
pub struct Connector {
    name: String,
    test_api_factory_name: String,
    pub capabilities: BitFlags<Capabilities>,
    pub tags: BitFlags<Tags>,
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
