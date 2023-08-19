use crate::postgres_datamodel_connector;
use psl_core::{datamodel_connector::Flavour, js_connector::JsConnector};

pub(crate) static PG_JS: JsConnector = JsConnector {
    flavour: Flavour::Postgres,
    canonical_connector: &postgres_datamodel_connector::PostgresDatamodelConnector,

    provider_name: "@prisma/pg",
    name: "node-postgres (pg) connector",
    allowed_protocols: Some(&["postgres", "postgresql"]),
};
