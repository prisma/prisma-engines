use crate::Connector;
use diagnostics::{DatamodelError, Span};

pub struct ConstraintNames;

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

    pub fn primary_key_name(table_name: &str, connector: &dyn Connector) -> String {
        let suffix = "_pkey";
        let limit = connector.max_identifier_length();

        let trimmed = if table_name.len() >= limit - 5 {
            table_name.split_at(limit - 5).0
        } else {
            table_name
        };

        format!("{}{}", trimmed, suffix)
    }

    pub fn unique_index_name(
        table_name: &str,
        column_names: &[Vec<(&str, Option<&str>)>],
        connector: &dyn Connector,
    ) -> String {
        const UNIQUE_SUFFIX: &str = "_key";
        Self::index_name_impl(table_name, column_names, UNIQUE_SUFFIX, connector)
    }

    pub fn non_unique_index_name(
        table_name: &str,
        column_names: &[Vec<(&str, Option<&str>)>],
        connector: &dyn Connector,
    ) -> String {
        const INDEX_SUFFIX: &str = "_idx";
        Self::index_name_impl(table_name, column_names, INDEX_SUFFIX, connector)
    }

    fn index_name_impl(
        table_name: &str,
        column_names: &[Vec<(&str, Option<&str>)>],
        suffix: &'static str,
        connector: &dyn Connector,
    ) -> String {
        let limit = connector.max_identifier_length();

        let mut out = String::with_capacity(table_name.len() + column_names.len() + suffix.len());

        out.push_str(table_name);
        out.push('_');

        let colnames = column_names
            .iter()
            .flatten()
            .map(|(i, _)| *i)
            .collect::<Vec<_>>()
            .join("_");

        out.push_str(&colnames);

        if out.len() >= limit - suffix.len() {
            out.truncate(limit - suffix.len());
        };

        out.push_str(suffix);

        out
    }

    pub fn default_name(table_name: &str, column_name: &str, connector: &dyn Connector) -> String {
        let limit = connector.max_identifier_length();
        let joined = format!("{}_{}", table_name, column_name);

        let trimmed = if joined.len() >= limit - 3 {
            joined.split_at(limit - 3).0
        } else {
            joined.as_str()
        };

        format!("{}_df", trimmed)
    }

    pub fn foreign_key_constraint_name(table_name: &str, column_names: &[&str], connector: &dyn Connector) -> String {
        let fk_suffix = "_fkey";
        let limit = connector.max_identifier_length();

        let joined = format!("{}_{}", table_name, column_names.join("_"));

        let trimmed = if joined.len() >= limit - 5 {
            joined.split_at(limit - 5).0
        } else {
            joined.as_str()
        };

        format!("{}{}", trimmed, fk_suffix)
    }

    pub fn is_db_name_too_long(
        span: Span,
        object_name: &str,
        name: Option<&str>,
        attribute: &str,
        connector: &dyn Connector,
        double_at: bool,
    ) -> Option<DatamodelError> {
        if let Some(name) = name {
            if name.len() > connector.max_identifier_length() {
                let ats = if double_at { "@@" } else { "@" };
                return Some(DatamodelError::new_model_validation_error(
                    &format!("The constraint name '{}' specified in the `map` argument for the `{}{}` constraint is too long for your chosen provider. The maximum allowed length is {} bytes.", name, ats, attribute, connector.max_identifier_length()),
                    object_name,
                    span,
                ));
            }
        }
        None
    }
}
