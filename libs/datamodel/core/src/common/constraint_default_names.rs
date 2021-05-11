use crate::IndexType;

pub struct ConstraintNames {}

impl ConstraintNames {
    ///Aligned with PG, to maximize the amount of times where we do not need
    ///to render names because they already align with our convention
    ///
    /// Postgres Naming conventions
    /// {tablename}_{columnname(s)}_{suffix}
    ///
    /// where the suffix is one of the following:
    ///
    /// pkey for a Primary Key constraint
    /// key for a Unique constraint
    /// excl for an Exclusion constraint
    /// idx for any other kind of index
    /// fkey for a Foreign key
    /// check for a Check constraint
    /// seq for sequences
    ///
    /// addditional for SQLSever
    /// dflt for Default Constraint

    pub fn primary_key_name(model: &str, fields: Vec<String>) -> String {
        format!("{}_{}_pkey", model, fields.join("_"))
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
