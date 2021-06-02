use crate::IndexType;
use datamodel_connector::Connector;

pub struct ConstraintNames {}

impl ConstraintNames {
    ///Aligned with PG, to maximize the amount of times where we do not need
    ///to render names because they already align with our convention.
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

    pub fn primary_key_name(table_name: &str, connector: Option<&dyn Connector>) -> String {
        let suffix = "_pkey";
        let limit = connector.map(|c| c.constraint_name_length()).unwrap_or(1000);

        let trimmed = if table_name.len() >= limit - 5 {
            table_name.split_at(limit - 5).0
        } else {
            table_name
        };

        format!("{}{}", trimmed, suffix)
    }

    pub fn index_name(
        table_name: &str,
        column_names: Vec<String>,
        tpe: IndexType,
        connector: Option<&dyn Connector>,
    ) -> String {
        let index_suffix = "_idx";
        let unique_suffix = "_key";
        let limit = connector.map(|c| c.constraint_name_length()).unwrap_or(1000);

        let joined = format!("{}_{}", table_name, column_names.join("_"));

        let trimmed = if joined.len() >= limit - 4 {
            joined.split_at(limit - 4).0
        } else {
            joined.as_str()
        };

        match tpe {
            IndexType::Unique => format!("{}{}", trimmed, unique_suffix),
            IndexType::Normal => format!("{}{}", trimmed, index_suffix),
        }
    }

    pub fn foreign_key_constraint_name(
        table_name: &str,
        column_names: Vec<String>,
        connector: Option<&dyn Connector>,
    ) -> String {
        let fk_suffix = "_fkey";
        let limit = connector.map(|c| c.constraint_name_length()).unwrap_or(1000);

        let joined = format!("{}_{}", table_name, column_names.join("_"));

        let trimmed = if joined.len() >= limit - 5 {
            joined.split_at(limit - 5).0
        } else {
            joined.as_str()
        };

        format!("{}{}", trimmed, fk_suffix)
    }

    pub fn default_constraint_name(table_name: &str, column_name: &str) -> String {
        format!("{}_{}_dflt", table_name, column_name)
    }
}
