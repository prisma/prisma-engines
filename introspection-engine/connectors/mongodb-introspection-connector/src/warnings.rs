use introspection_connector::Warning;
use serde_json::json;

pub(crate) fn explicit_id_column(affected: &str) -> Warning {
    Warning {
        code: 101,
        message: "The given model has a field with the name `id`, that clashes with the primary key. Please rename either one of them before using the data model.".into(),
        affected: json![{"name": affected}],
    }
}
