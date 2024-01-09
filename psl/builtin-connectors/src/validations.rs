use psl_core::{
    datamodel_connector::{Connector, Flavour, RelationMode},
    diagnostics::Diagnostics,
    parser_database::walkers,
};

use crate::{
    cockroach_datamodel_connector, mongodb, mssql_datamodel_connector, mysql_datamodel_connector,
    postgres_datamodel_connector,
};

pub(super) fn validate_model(
    connector: &dyn Connector,
    model: walkers::ModelWalker<'_>,
    relation_mode: RelationMode,
    errors: &mut Diagnostics,
) {
    match connector.flavour() {
        Flavour::Postgres => postgres_datamodel_connector::validations::validate_model(connector, model, errors),
        Flavour::Mysql => {
            mysql_datamodel_connector::validations::validate_model(connector, model, relation_mode, errors)
        }
        Flavour::Cockroach => cockroach_datamodel_connector::validations::validate_model(model, errors),
        Flavour::Sqlserver => mssql_datamodel_connector::validations::validate_model(connector, model, errors),

        Flavour::Sqlite => {}
        Flavour::Mongo => mongodb::validations::validate_model(model, errors),
    }
}
