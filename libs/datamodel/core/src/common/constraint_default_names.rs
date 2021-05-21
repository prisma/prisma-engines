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

    pub fn primary_key_name(model: &str) -> String {
        format!("{}_pkey", model)
    }

    pub fn index_name(model: &str, fields: Vec<String>, tpe: IndexType) -> String {
        match tpe {
            IndexType::Unique => format!("{}_{}_key", model, fields.join("_")),
            IndexType::Normal => format!("{}_{}_idx", model, fields.join("_")),
        }
    }

    pub fn foreign_key_constraint_name(model: &str, fields: Vec<String>) -> String {
        format!("{}_{}_fkey", model, fields.join("_"))
    }

    pub fn default_constraint_name(model: &str, field: &str) -> String {
        format!("{}_{}_dflt", model, field)
    }
}
