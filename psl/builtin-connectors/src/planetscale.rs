use crate::mysql_datamodel_connector;
use psl_core::{datamodel_connector::Flavour, js_connector::JsConnector};

pub(crate) static PLANETSCALE_SERVERLESS: JsConnector = JsConnector {
    flavour: Flavour::Mysql,
    canonical_connector: &mysql_datamodel_connector::MySqlDatamodelConnector,

    provider_name: "@prisma/planetscale",
    name: "planetscale serverless",
    allowed_protocols: Some(&["mysql", "https", "mysqls"]),
};
