use crate::{IndexType, WithDatabaseName};
use datamodel_connector::Connector;
use dml::model::{IndexDefinition, Model};

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

    // pub fn primary_key_name_matches(
    //     constraint_name: Option<String>,
    //     table_name: &str,
    //     connector: &dyn Connector,
    // ) -> bool {
    //     constraint_name == ConstraintNames::primary_key_name(table_name, connector)
    // }
    //
    // pub fn primary_key_name(table_name: &str, connector: &dyn Connector) -> Option<String> {
    //     let suffix = "_pkey";
    //     let limit = connector.constraint_name_length();
    //
    //     let trimmed = if table_name.len() >= limit - 5 {
    //         table_name.split_at(limit - 5).0
    //     } else {
    //         table_name
    //     };
    //
    //     Some(format!("{}{}", trimmed, suffix))
    // }

    pub fn index_name_matches(idx: &IndexDefinition, model: &Model, connector: &dyn Connector) -> bool {
        let column_names: Vec<&str> = idx
            .fields
            .iter()
            .map(|field_name| model.find_scalar_field(field_name).unwrap().final_database_name())
            .collect();

        idx.db_name.as_ref().unwrap()
            == &ConstraintNames::index_name(model.final_database_name(), &column_names, idx.tpe, connector)
    }

    pub fn index_name(table_name: &str, column_names: &[&str], tpe: IndexType, connector: &dyn Connector) -> String {
        let index_suffix = "_idx";
        let unique_suffix = "_key";
        let limit = connector.constraint_name_length();

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

    // pub fn foreign_key_constraint_name(
    //     table_name: &str,
    //     column_names: Vec<String>,
    //     connector: &dyn Connector,
    // ) -> String {
    //     let fk_suffix = "_fkey";
    //     let limit = connector.constraint_name_length();
    //
    //     let joined = format!("{}_{}", table_name, column_names.join("_"));
    //
    //     let trimmed = if joined.len() >= limit - 5 {
    //         joined.split_at(limit - 5).0
    //     } else {
    //         joined.as_str()
    //     };
    //
    //     format!("{}{}", trimmed, fk_suffix)
    // }

    // pub fn default_constraint_name(table_name: &str, column_name: &str) -> String {
    //     format!("{}_{}_dflt", table_name, column_name)
    // }
}
