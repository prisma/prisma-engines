pub mod test_api;

use barrel::Migration;
use quaint::{prelude::Queryable, single::Quaint};
use test_setup::{BitFlags, Tags};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
pub type TestResult = Result<()>;

#[macro_export]
macro_rules! assert_eq_schema {
    ($left:expr, $right:expr) => {
        let no_whitespace_expected = $left.replace(' ', "").replace('\n', "");
        let no_whitespace_result = $right.replace(' ', "").replace('\n', "");

        pretty_assertions::assert_eq!(no_whitespace_result, no_whitespace_expected);
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
    tags: BitFlags<Tags>,
}

impl BarrelMigrationExecutor {
    pub async fn execute<F>(&self, migration_fn: F) -> TestResult
    where
        F: FnOnce(&mut Migration),
    {
        self.execute_with_schema(migration_fn, &self.schema_name).await?;

        Ok(())
    }

    pub async fn execute_with_schema<F>(&self, migration_fn: F, schema_name: &str) -> TestResult
    where
        F: FnOnce(&mut Migration),
    {
        let mut migration = if self.tags.intersects(Tags::Vitess) {
            Migration::new()
        } else {
            Migration::new().schema(schema_name)
        };

        migration_fn(&mut migration);

        let full_sql = migration.make_from(self.sql_variant);

        if full_sql.is_empty() {
            return Ok(());
        }

        self.database.raw_cmd(&full_sql).await?;

        Ok(())
    }
}
