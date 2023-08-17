use crate::postgres_datamodel_connector;
use psl_core::{datamodel_connector::Flavour, js_connector::JsConnector};

pub(crate) static NEON_SERVERLESS: JsConnector = JsConnector {
    flavour: Flavour::Postgres,
    canonical_connector: &postgres_datamodel_connector::PostgresDatamodelConnector,

    provider_name: "@prisma/neon",
    name: "neon serverless (pg-compatible)",
    allowed_protocols: Some(&["postgres"]),
};
