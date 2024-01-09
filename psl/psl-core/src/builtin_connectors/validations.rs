use diagnostics::{DatamodelError, Diagnostics, Span};
use parser_database::{walkers, ScalarType};

use crate::datamodel_connector::{Connector, Flavour, NativeTypeInstance, RelationMode};

use super::{
    cockroach_datamodel_connector::validations as cockroach, mongodb::validations as mongodb,
    mssql_datamodel_connector::validations as mssql, mysql_datamodel_connector::validations as mysql,
    postgres_datamodel_connector::validations as postgres,
};

pub(crate) fn validate_enum(connector: &dyn Connector, r#enum: walkers::EnumWalker<'_>, diagnostics: &mut Diagnostics) {
    match connector.flavour() {
        Flavour::Mysql => mysql::validate_enum(r#enum, diagnostics),
        _ => {}
    }
}

pub(crate) fn validate_model(
    connector: &dyn Connector,
    model: walkers::ModelWalker<'_>,
    relation_mode: RelationMode,
    diagnostics: &mut Diagnostics,
) {
    match connector.flavour() {
        Flavour::Cockroach => cockroach::validate_model(model, diagnostics),
        Flavour::Mongo => mongodb::validate_model(model, diagnostics),
        Flavour::Sqlserver => mssql::validate_model(connector, model, diagnostics),
        Flavour::Mysql => mysql::validate_model(connector, model, relation_mode, diagnostics),

        Flavour::Postgres => postgres::validate_model(connector, model, diagnostics),
        Flavour::Sqlite => {}
    }
}

pub(crate) fn validate_relation_field(
    connector: &dyn Connector,
    field: crate::parser_database::walkers::RelationFieldWalker<'_>,
    errors: &mut Diagnostics,
) {
    match connector.flavour() {
        Flavour::Mongo => mongodb::validate_relation_field(field, errors),
        _ => {}
    }
}

pub(crate) fn validate_scalar_field_unknown_default_functions(
    connector: &dyn Connector,
    db: &parser_database::ParserDatabase,
    diagnostics: &mut Diagnostics,
) {
    match connector.flavour() {
        Flavour::Cockroach => cockroach::validate_scalar_field_unknown_default_functions(db, diagnostics),
        _ => {
            for d in db.walk_scalar_field_defaults_with_unknown_function() {
                let (func_name, _, span) = d.value().as_function().unwrap();
                diagnostics.push_error(DatamodelError::new_default_unknown_function(func_name, span));
            }
        }
    }
}

pub(crate) fn validate_native_type_arguments(
    connector: &dyn Connector,
    native_type_instance: &NativeTypeInstance,
    scalar_type: &ScalarType,
    span: Span,
    errors: &mut Diagnostics,
) {
    match connector.flavour() {
        Flavour::Cockroach => cockroach::validate_native_type_arguments(connector, native_type_instance, span, errors),
        Flavour::Sqlserver => mssql::validate_native_type_arguments(connector, native_type_instance, span, errors),
        Flavour::Mysql => {
            mysql::validate_native_type_arguments(connector, native_type_instance, scalar_type, span, errors)
        }
        Flavour::Postgres => postgres::validate_native_type_arguments(connector, native_type_instance, span, errors),
        Flavour::Sqlite | Flavour::Mongo => {}
    }
}
