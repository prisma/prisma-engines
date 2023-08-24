use crate::sqlite_datamodel_connector;
use psl_core::{datamodel_connector::Flavour, js_connector::JsConnector};

pub(crate) static LIBSQL: JsConnector = JsConnector {
    flavour: Flavour::Sqlite,
    canonical_connector: &sqlite_datamodel_connector::SqliteDatamodelConnector,

    provider_name: "@prisma/libsql",
    name: "libSQL (Turso) connector",
    allowed_protocols: Some(&["https", "http"]),
};
