use crate::IndexType;

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
    /// addditional for SQLServer:
    /// dflt for Default Constraint
    ///
    /// not used for now:
    /// check for a Check constraint
    /// excl for an Exclusion constraint
    /// seq for sequences

    pub fn primary_key_name(table_name: &str) -> String {
        format!("{}_pkey", table_name)
    }

    pub fn index_name(table_name: &str, column_names: Vec<String>, tpe: IndexType) -> String {
        match tpe {
            IndexType::Unique => format!("{}_{}_key", table_name, column_names.join("_")),
            IndexType::Normal => format!("{}_{}_idx", table_name, column_names.join("_")),
        }
    }

    pub fn foreign_key_constraint_name(table_name: &str, column_names: Vec<String>) -> String {
        format!("{}_{}_fkey", table_name, column_names.join("_"))
    }

    pub fn default_constraint_name(table_name: &str, column_name: &str) -> String {
        format!("{}_{}_dflt", table_name, column_name)
    }
}
