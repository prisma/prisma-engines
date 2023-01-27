use crate::datamodel_connector::Connector;
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

        let table_name = if table_name.len() >= limit - 5 {
            let split = floor_char_boundary(table_name, limit - 5);

            table_name.split_at(split).0
        } else {
            table_name
        };

        format!("{table_name}{suffix}")
    }

    pub fn unique_index_name(table_name: &str, column_names: &[&str], connector: &dyn Connector) -> String {
        const UNIQUE_SUFFIX: &str = "_key";

        Self::index_name_impl(table_name, column_names, UNIQUE_SUFFIX, connector)
    }

    pub fn non_unique_index_name(table_name: &str, column_names: &[&str], connector: &dyn Connector) -> String {
        const INDEX_SUFFIX: &str = "_idx";

        Self::index_name_impl(table_name, column_names, INDEX_SUFFIX, connector)
    }

    fn index_name_impl(
        table_name: &str,
        column_names: &[&str],
        suffix: &'static str,
        connector: &dyn Connector,
    ) -> String {
        let limit = connector.max_identifier_length();

        let mut out = String::with_capacity(table_name.len() + column_names.len() + suffix.len());

        out.push_str(table_name);
        out.push('_');
        out.push_str(&column_names.join("_"));

        if out.len() >= limit - suffix.len() {
            let split = floor_char_boundary(&out, limit - suffix.len());
            out.truncate(split);
        };

        out.push_str(suffix);

        out
    }

    pub fn default_name(table_name: &str, column_name: &str, connector: &dyn Connector) -> String {
        let limit = connector.max_identifier_length();
        let mut joined = format!("{table_name}_{column_name}");

        if joined.len() >= limit - 3 {
            let split = floor_char_boundary(&joined, limit - 3);
            joined.truncate(split);
        }

        format!("{joined}_df")
    }

    /// Params:
    ///
    /// - table_name: the name of the _constrained_/_referencing_ table, not the referenced one.
    /// - column names: the _constrained_ column names
    pub fn foreign_key_constraint_name(table_name: &str, column_names: &[&str], connector: &dyn Connector) -> String {
        let fk_suffix = "_fkey";
        let limit = connector.max_identifier_length();

        let mut joined = format!("{table_name}_{}", column_names.join("_"));

        if joined.len() >= limit - 5 {
            let split = floor_char_boundary(&joined, limit - 5);
            joined.truncate(split);
        }

        format!("{joined}{fk_suffix}")
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
                    &format!("The constraint name '{name}' specified in the `map` argument for the `{ats}{attribute}` constraint is too long for your chosen provider. The maximum allowed length is {} bytes.", connector.max_identifier_length()),
                    "model",
                    object_name,
                    span,
                ));
            }
        }
        None
    }
}

/// Finds the closest `x` not exceeding `index` where
/// `is_char_boundary(x) is `true.
///
/// This method can help you to truncate a string so that it's still
/// valid UTF-8, but doesn't exceed a given number of bytes.
///
/// To be replaced with `std::str::floor_char_boundary` when it's
/// stabilized.
fn floor_char_boundary(s: &str, mut index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        while !s.is_char_boundary(index) {
            index -= 1;
        }

        index
    }
}
