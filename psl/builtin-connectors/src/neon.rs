use crate::postgres_datamodel_connector;
use psl_core::{
    datamodel_connector::RelationMode,
    js_connector::{Flavor, JsConnector},
};

pub(crate) static NEON_SERVERLESS: JsConnector = JsConnector {
    flavor: Flavor::Postgres,
    canonical_connector: &postgres_datamodel_connector::PostgresDatamodelConnector,

    provider_name: "@prisma/neon",
    name: "neon serverless (pg-compatible)",
    enforced_relation_mode: Some(RelationMode::Prisma),
    allowed_protocols: Some(&["postgres"]),
};
