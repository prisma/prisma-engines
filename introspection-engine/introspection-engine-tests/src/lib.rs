pub mod test_api;

use barrel::Migration;
use eyre::Result;
use quaint::{prelude::Queryable, single::Quaint};

#[macro_export]
macro_rules! assert_eq_datamodels {
    ($left:expr, $right:expr) => {
        let parsed_expected = datamodel::parse_datamodel($left).unwrap().subject;
        let parsed_result = datamodel::parse_datamodel($right).unwrap().subject;

        let reformatted_expected = datamodel::render_datamodel_to_string(&parsed_expected);
        let reformatted_result = datamodel::render_datamodel_to_string(&parsed_result);

        pretty_assertions::assert_eq!(reformatted_result, reformatted_expected);
    };
}

/// Left side should be `serde_json::Value` and the right side a string that can
/// be converted to JSON.
#[macro_export]
macro_rules! assert_eq_json {
    ($expected:expr, $result:expr) => {
        let val: serde_json::Value =
            serde_json::from_str($result.as_str()).expect("The right side value was not valid JSON.");

        pretty_assertions::assert_eq!($expected, val);
    };
}

pub struct BarrelMigrationExecutor {
    database: Quaint,
    sql_variant: barrel::backend::SqlVariant,
    schema_name: String,
}

impl BarrelMigrationExecutor {
    pub async fn execute<F>(&self, migration_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Migration),
    {
        self.execute_with_schema(migration_fn, &self.schema_name).await?;

        Ok(())
    }

    pub async fn execute_with_schema<F>(&self, migration_fn: F, schema_name: &str) -> Result<()>
    where
        F: FnOnce(&mut Migration),
    {
        let mut migration = Migration::new().schema(schema_name);
        migration_fn(&mut migration);

        let full_sql = migration.make_from(self.sql_variant);

        if full_sql.is_empty() {
            return Ok(());
        }

        self.database.raw_cmd(&full_sql).await?;

        Ok(())
    }
}
