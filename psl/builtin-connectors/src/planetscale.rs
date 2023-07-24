use crate::mysql_datamodel_connector;
use psl_core::{
    datamodel_connector::RelationMode,
    js_connector::{Flavor, JsConnector},
};

pub(crate) static PLANETSCALE_SERVERLESS: JsConnector = JsConnector {
    flavor: Flavor::MySQL,
    canonical_connector: &mysql_datamodel_connector::MySqlDatamodelConnector,

    provider_name: "@prisma/planetscale",
    name: "planetscale serverless",
    enforced_relation_mode: Some(RelationMode::Prisma),
    allowed_protocols: Some(&["mysql", "https", "mysqls"]),
};
