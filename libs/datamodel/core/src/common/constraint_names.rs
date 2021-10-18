use crate::ast::Span;
use crate::diagnostics::DatamodelError;
use crate::{PrimaryKeyDefinition, WithDatabaseName};
use datamodel_connector::Connector;
use dml::model::{IndexDefinition, Model};
use dml::relation_info::RelationInfo;
use once_cell::sync::Lazy;
use regex::Regex;

pub(crate) struct ConstraintNames;

impl ConstraintNames {
    /// Aligned with PG, to maximize the amount of times where we do not need
    /// to render names because they already align with our convention.
    ///
    /// We always take the database names of entities. So if a model is remapped to
    /// a different name in the datamodel, the default name generation will still take
    /// the name of the table in the db as input. Same goes for fields / columns.
    ///
    /// Postgres Naming conventions
    ///
    /// Without column names {tablename}_{suffix}
    /// pkey for a Primary Key constraint
    ///
    /// Including column names: {tablename}_{columnname(s)}_{suffix}
    /// column names are joined with an _ if there are multiple
    /// key for a Unique constraint
    /// idx for any other kind of index
    /// fkey for a Foreign key
    ///
    /// additional for SQLServer:
    /// dflt for Default Constraint
    ///
    /// not used for now:
    /// check for a Check constraint
    /// excl for an Exclusion constraint
    /// seq for sequences
    ///

    pub(crate) fn primary_key_name_matches(
        pk: &PrimaryKeyDefinition,
        model: &Model,
        connector: &dyn Connector,
    ) -> bool {
        pk.db_name.as_ref().unwrap() == &ConstraintNames::primary_key_name(model.final_database_name(), connector)
    }

    pub(crate) fn primary_key_name(table_name: &str, connector: &dyn Connector) -> String {
        let suffix = "_pkey";
        let limit = connector.constraint_name_length();

        let trimmed = if table_name.len() >= limit - 5 {
            table_name.split_at(limit - 5).0
        } else {
            table_name
        };

        format!("{}{}", trimmed, suffix)
    }

    pub(crate) fn index_name_matches(idx: &IndexDefinition, model: &Model, connector: &dyn Connector) -> bool {
        let column_names: Vec<&str> = idx
            .fields
            .iter()
            .map(|field_name| model.find_scalar_field(field_name).unwrap().final_database_name())
            .collect();

        let expected = if idx.is_unique() {
            Self::unique_index_name(model.final_database_name(), &column_names, connector)
        } else {
            Self::non_unique_index_name(model.final_database_name(), &column_names, connector)
        };

        idx.db_name.as_deref().unwrap() == expected
    }

    pub(crate) fn unique_index_name(table_name: &str, column_names: &[&str], connector: &dyn Connector) -> String {
        const UNIQUE_SUFFIX: &str = "_key";
        Self::index_name_impl(table_name, column_names, UNIQUE_SUFFIX, connector)
    }

    pub(crate) fn non_unique_index_name(table_name: &str, column_names: &[&str], connector: &dyn Connector) -> String {
        const INDEX_SUFFIX: &str = "_idx";
        Self::index_name_impl(table_name, column_names, INDEX_SUFFIX, connector)
    }

    fn index_name_impl(
        table_name: &str,
        column_names: &[&str],
        suffix: &'static str,
        connector: &dyn Connector,
    ) -> String {
        let limit = connector.constraint_name_length();

        let mut out = String::with_capacity(table_name.len() + column_names.len() + suffix.len());

        out.push_str(table_name);
        out.push('_');

        let colnames = column_names.join("_");

        out.push_str(&colnames);

        if out.len() >= limit - suffix.len() {
            out.truncate(limit - suffix.len());
        };

        out.push_str(suffix);

        out
    }

    pub(crate) fn default_name(table_name: &str, column_name: &str, connector: &dyn Connector) -> String {
        let limit = connector.constraint_name_length();
        let joined = format!("{}_{}", table_name, column_name);

        let trimmed = if joined.len() >= limit - 3 {
            joined.split_at(limit - 3).0
        } else {
            joined.as_str()
        };

        format!("{}_df", trimmed)
    }

    pub(crate) fn foreign_key_name_matches(ri: &RelationInfo, model: &Model, connector: &dyn Connector) -> bool {
        let column_names: Vec<&str> = ri
            .fields
            .iter()
            .map(|field_name| {
                // We cannot unwrap here, due to us re-introspecting relations
                // and if we're not using foreign keys, we might copy a relation
                // that is not valid anymore. We still want to write that to the
                // file and let user fix it, but if we unwrap here, we will
                // panic.
                model
                    .find_scalar_field(field_name)
                    .map(|field| field.final_database_name())
                    .unwrap_or(field_name)
            })
            .collect();

        ri.fk_name.as_ref().unwrap()
            == &ConstraintNames::foreign_key_constraint_name(model.final_database_name(), &column_names, connector)
    }

    pub(crate) fn foreign_key_constraint_name(
        table_name: &str,
        column_names: &[&str],
        connector: &dyn Connector,
    ) -> String {
        let fk_suffix = "_fkey";
        let limit = connector.constraint_name_length();

        let joined = format!("{}_{}", table_name, column_names.join("_"));

        let trimmed = if joined.len() >= limit - 5 {
            joined.split_at(limit - 5).0
        } else {
            joined.as_str()
        };

        format!("{}{}", trimmed, fk_suffix)
    }

    pub(crate) fn is_client_name_valid(
        span: Span,
        object_name: &str,
        name: Option<&str>,
        attribute: &str,
    ) -> Option<DatamodelError> {
        //only Alphanumeric characters and underscore are allowed due to this making its way into the client API
        //todo what about starting with a number or underscore?
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

        if let Some(name) = name {
            if RE.is_match(name) {
                return  Some(DatamodelError::new_model_validation_error(
                    &format!("The `name` property within the `{}` attribute only allows for the following characters: `_a-zA-Z0-9`.", attribute),
                    object_name,
                    span,
                ));
            }
        }
        None
    }

    pub(crate) fn is_db_name_too_long(
        span: Span,
        object_name: &str,
        name: Option<&str>,
        attribute: &str,
        connector: &dyn Connector,
    ) -> Option<DatamodelError> {
        if let Some(name) = name {
            if name.len() > connector.constraint_name_length() {
                return Some(DatamodelError::new_model_validation_error(
                    &format!("The constraint name '{}' specified in the `map` argument for the `{}` constraint is too long for your chosen provider. The maximum allowed length is {} bytes.", name, attribute, connector.constraint_name_length()),
                    object_name,
                    span,
                ));
            }
        }
        None
    }
}
