//! Replaying a migration history into a shadow database the user provided.
//!
//! Such a database is reset before the history is replayed into it, which destroys whatever it
//! holds, so it is only ever used when it holds nothing, or when the user said that its contents
//! can go and there is little enough of them for that to be a plausible thing to have said. It is
//! also left empty afterwards, which is what makes the promise hold for the next replay: a command
//! that replays the history several times finds an empty database every time after the first, and
//! so does the command after it.

use crate::flavour::{SqlConnector, UsingExternalShadowDb};
use quaint::ast::{Aliasable, Select, Table, asterisk, count};
use schema_connector::{ConnectorError, ConnectorResult, Namespaces, SchemaFilter, migrations_directory::Migrations};
use sql_schema_describer::SqlSchema;
use user_facing_errors::schema_engine::{ShadowDbNotEmpty, ShadowDbTooMuchData};

/// How much data a shadow database may hold and still be reset. A database holding more than this
/// is not the throwaway one the command expects, whoever says otherwise: consent is given to a
/// command in a terminal, and it cannot mean "and also destroy this database I forgot about".
const ROW_COUNT_LIMIT: u64 = 1000;

/// Replays `migrations` into the shadow database `shadow_db` is connected to, and leaves it empty.
///
/// `reset_allowed` carries the user's consent to destroy the contents of a shadow database that is
/// not empty; `location` names the database in the error raised when that consent is missing, and
/// must be safe to show (see [`crate::sanitize_connection_string`]).
///
/// Disposing of `shadow_db` is the caller's business, on this function's success and failure alike.
pub(crate) async fn replay_migration_history(
    shadow_db: &mut (dyn SqlConnector + Send + Sync),
    migrations: &Migrations,
    namespaces: Option<Namespaces>,
    filter: &SchemaFilter,
    reset_allowed: bool,
    location: &str,
) -> ConnectorResult<SqlSchema> {
    if reset_allowed {
        ensure_shadow_db_is_small_enough_to_reset(shadow_db, namespaces.clone(), location).await?;

        // Consent is exercised here rather than left to the flavour: SQLite and the Wasm PostgreSQL
        // connector replay a migration history into an external shadow database without resetting
        // it first, and the contents the user agreed to part with would collide with the replay.
        reset(shadow_db, namespaces.clone(), filter).await?;
    } else {
        ensure_shadow_db_is_empty(shadow_db, namespaces.clone(), location).await?;
    }

    let schema = shadow_db
        .sql_schema_from_migration_history(migrations, namespaces.clone(), filter, UsingExternalShadowDb::Yes)
        .await?;

    reset(shadow_db, namespaces, filter).await?;

    Ok(schema)
}

/// Empties the shadow database. A user may well have granted the engine the right to create and to
/// drop tables in it without making it the owner of anything, which is not enough to drop and
/// recreate the schema itself, so the objects are dropped one by one when that fails.
async fn reset(
    shadow_db: &mut (dyn SqlConnector + Send + Sync),
    namespaces: Option<Namespaces>,
    filter: &SchemaFilter,
) -> ConnectorResult<()> {
    if shadow_db.reset(namespaces.clone()).await.is_err() {
        crate::best_effort_reset(shadow_db, namespaces, filter).await?;
    }

    Ok(())
}

/// A failed replay leaves the shadow database as it was when it failed: the next run finding it
/// dirty, and asking about it, is the signal that something went wrong here.
async fn ensure_shadow_db_is_empty(
    shadow_db: &mut (dyn SqlConnector + Send + Sync),
    namespaces: Option<Namespaces>,
    location: &str,
) -> ConnectorResult<()> {
    let schema = shadow_db.describe_schema(namespaces).await?;

    if holds_no_objects(&schema) {
        return Ok(());
    }

    Err(ConnectorError::user_facing(ShadowDbNotEmpty {
        shadow_database_location: location.to_owned(),
    }))
}

/// Whether the described schema holds nothing at all — no table (`_prisma_migrations` included, a
/// database that only holds one is the remains of an earlier replay), no view, no enum, no
/// user-defined type, no stored procedure.
fn holds_no_objects(schema: &SqlSchema) -> bool {
    schema.tables_count() == 0
        && schema.views_count() == 0
        && schema.enum_walkers().len() == 0
        && schema.udt_walkers().next().is_none()
        && schema.procedures_count() == 0
}

/// Refuses a shadow database that holds more data than consent covers.
///
/// The count is bounded by construction: each table contributes at most one more row than the
/// budget left for it, so the whole check reads at most `ROW_COUNT_LIMIT + <number of tables>`
/// rows however large the database is, and stops as soon as the answer is settled. A count that
/// cannot be taken is not an answer: the error travels, and the database is left alone.
async fn ensure_shadow_db_is_small_enough_to_reset(
    shadow_db: &mut (dyn SqlConnector + Send + Sync),
    namespaces: Option<Namespaces>,
    location: &str,
) -> ConnectorResult<()> {
    let schema = shadow_db.describe_schema(namespaces).await?;

    if holds_no_objects(&schema) {
        return Ok(());
    }

    // Views are derived from the tables that are counted, and the other object classes hold no
    // rows at all.
    let tables: Vec<(Option<String>, String)> = schema
        .table_walkers()
        .map(|table| (table.namespace().map(ToOwned::to_owned), table.name().to_owned()))
        .collect();

    let mut budget = RowBudget::new(ROW_COUNT_LIMIT);

    for (namespace, table) in tables {
        let Some(limit) = budget.query_limit() else {
            break;
        };

        let rows = count_rows_up_to(shadow_db, namespace.as_deref(), &table, limit).await?;

        if budget.add(rows) {
            return Err(ConnectorError::user_facing(ShadowDbTooMuchData {
                shadow_database_location: location.to_owned(),
                row_count_limit: ROW_COUNT_LIMIT,
            }));
        }
    }

    Ok(())
}

/// How many rows a table holds, counted no further than `limit`.
///
/// The limit is inside the query — `SELECT COUNT(*) FROM (SELECT 1 FROM <table> LIMIT <limit>)` —
/// so the database stops reading once it has enough to answer, rather than counting a table that
/// may hold millions of rows in order to learn that it holds more than a thousand.
async fn count_rows_up_to(
    shadow_db: &mut (dyn SqlConnector + Send + Sync),
    namespace: Option<&str>,
    table: &str,
    limit: u64,
) -> ConnectorResult<u64> {
    let result = shadow_db
        .query(count_rows_query(namespace, table, limit).into())
        .await?;

    row_count_from(result).ok_or_else(|| {
        let table = match namespace {
            Some(namespace) => format!("{namespace}.{table}"),
            None => table.to_owned(),
        };

        ConnectorError::from_msg(format!(
            "Failed to read the number of rows in `{table}` from the shadow database."
        ))
    })
}

/// The count in the first column of the first row, when it is there and is a number of rows.
/// `None` stands for an answer that cannot be read, which is not the same as an answer of zero:
/// a table that cannot be counted is not a table that is known to be empty.
fn row_count_from(result: quaint::prelude::ResultSet) -> Option<u64> {
    let count = result.into_iter().next()?.into_iter().next()?.as_integer()?;

    u64::try_from(count).ok()
}

/// `SELECT COUNT(*) FROM (SELECT 1 FROM <table> LIMIT <limit>)`, rendered for whichever database
/// the query is sent to — the limit is a limit on SQL Server too, spelled the way it spells it.
fn count_rows_query<'a>(namespace: Option<&str>, table: &str, limit: u64) -> Select<'a> {
    let table = match namespace {
        Some(namespace) => Table::from((namespace.to_owned(), table.to_owned())),
        None => Table::from(table.to_owned()),
    };

    let limited_rows = Select::from_table(table).value(1).limit(limit as usize);

    Select::from_table(Table::from(limited_rows).alias("limited_rows")).value(count(asterisk()))
}

/// Tracks how much of the row budget is left, and says when it is gone.
#[derive(Debug)]
struct RowBudget {
    limit: u64,
    counted: u64,
}

impl RowBudget {
    fn new(limit: u64) -> Self {
        RowBudget { limit, counted: 0 }
    }

    /// How far the next table needs to be counted: one row past what is left of the budget, which
    /// is what tells "the budget is exactly used up" apart from "the budget is exceeded".
    fn query_limit(&self) -> Option<u64> {
        (self.counted <= self.limit).then(|| self.limit - self.counted + 1)
    }

    /// Adds a table's rows to the total, and returns whether the budget is now exceeded.
    fn add(&mut self, rows: u64) -> bool {
        self.counted = self.counted.saturating_add(rows);
        self.counted > self.limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quaint::prelude::Value;

    fn schema_with_a_namespace() -> SqlSchema {
        let mut schema = SqlSchema::default();
        schema.push_namespace("public".to_owned());
        schema
    }

    #[test]
    fn a_schema_without_objects_holds_no_objects() {
        assert!(holds_no_objects(&SqlSchema::default()));
        assert!(holds_no_objects(&schema_with_a_namespace()));
    }

    #[test]
    fn a_table_counts() {
        let mut schema = schema_with_a_namespace();
        schema.push_table("Cat".to_owned(), Default::default(), None);

        assert!(!holds_no_objects(&schema));
    }

    #[test]
    fn the_migrations_table_alone_counts() {
        let mut schema = schema_with_a_namespace();
        schema.push_table(crate::MIGRATIONS_TABLE_NAME.to_owned(), Default::default(), None);

        assert!(!holds_no_objects(&schema));
    }

    #[test]
    fn a_view_counts() {
        let mut schema = schema_with_a_namespace();
        schema.push_view("Cats".to_owned(), Default::default(), None, None);

        assert!(!holds_no_objects(&schema));
    }

    #[test]
    fn an_enum_counts() {
        let mut schema = schema_with_a_namespace();
        schema.push_enum(Default::default(), "Mood".to_owned(), None);

        assert!(!holds_no_objects(&schema));
    }

    #[test]
    fn a_user_defined_type_counts() {
        let mut schema = schema_with_a_namespace();
        schema.push_udt(Default::default(), "Whiskers".to_owned(), None);

        assert!(!holds_no_objects(&schema));
    }

    #[test]
    fn the_first_table_may_be_counted_one_row_past_the_limit() {
        let budget = RowBudget::new(1000);

        assert_eq!(budget.query_limit(), Some(1001));
    }

    #[test]
    fn each_table_is_counted_only_as_far_as_the_budget_left_for_it() {
        let mut budget = RowBudget::new(1000);

        assert!(!budget.add(400));
        assert_eq!(budget.query_limit(), Some(601));

        assert!(!budget.add(600));
        // The budget is used up to the last row, which is not one row too many.
        assert_eq!(budget.query_limit(), Some(1));
    }

    #[test]
    fn the_limit_itself_is_not_too_much_data() {
        let mut budget = RowBudget::new(1000);

        assert!(!budget.add(1000));
    }

    #[test]
    fn one_row_past_the_limit_is_too_much_data() {
        let mut budget = RowBudget::new(1000);

        assert!(budget.add(1001));
    }

    #[test]
    fn nothing_more_is_counted_once_the_budget_is_gone() {
        let mut budget = RowBudget::new(1000);

        assert!(budget.add(2000));
        assert_eq!(budget.query_limit(), None);
    }

    fn count_result(values: Vec<quaint::prelude::Value<'static>>) -> quaint::prelude::ResultSet {
        let rows = if values.is_empty() { Vec::new() } else { vec![values] };

        quaint::prelude::ResultSet::new(vec!["count".to_owned()], vec![quaint::prelude::ColumnType::Int64], rows)
    }

    #[test]
    fn a_count_is_read_however_the_database_sizes_it() {
        // PostgreSQL, MySQL and SQLite answer with a 64-bit integer, SQL Server with a 32-bit one.
        assert_eq!(row_count_from(count_result(vec![Value::int64(7)])), Some(7));
        assert_eq!(row_count_from(count_result(vec![Value::int32(7)])), Some(7));
    }

    #[test]
    fn a_count_that_cannot_be_read_is_not_a_count_of_zero() {
        assert_eq!(row_count_from(count_result(vec![])), None);
        assert_eq!(row_count_from(count_result(vec![Value::null_int64()])), None);
        assert_eq!(row_count_from(count_result(vec![Value::text("seven")])), None);
        assert_eq!(row_count_from(count_result(vec![Value::int64(-1)])), None);
    }

    #[test]
    fn a_table_of_its_own_cannot_overflow_the_total() {
        let mut budget = RowBudget::new(1000);

        assert!(budget.add(u64::MAX));
        assert!(budget.add(u64::MAX));
        assert_eq!(budget.query_limit(), None);
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn the_count_is_limited_in_the_query_itself() {
        use quaint::visitor::Visitor;

        let (sql, _) = quaint::visitor::Sqlite::build(count_rows_query(None, "Cat", 1001)).unwrap();

        assert_eq!(
            sql,
            "SELECT COUNT(*) FROM (SELECT ? FROM `Cat` LIMIT ?) AS `limited_rows`"
        );
    }

    #[cfg(feature = "postgresql")]
    #[test]
    fn the_table_is_counted_in_its_own_namespace() {
        use quaint::visitor::Visitor;

        let (sql, _) = quaint::visitor::Postgres::build(count_rows_query(Some("public"), "Cat", 1001)).unwrap();

        assert_eq!(
            sql,
            "SELECT COUNT(*) FROM (SELECT $1 FROM \"public\".\"Cat\" LIMIT $2) AS \"limited_rows\""
        );
    }

    #[cfg(feature = "mysql")]
    #[test]
    fn mysql_gets_its_own_quoting_and_limit() {
        use quaint::visitor::Visitor;

        let (sql, _) = quaint::visitor::Mysql::build(count_rows_query(None, "Cat", 1001)).unwrap();

        assert_eq!(
            sql,
            "SELECT COUNT(*) FROM (SELECT ? FROM `Cat` LIMIT ?) AS `limited_rows`"
        );
    }

    #[cfg(feature = "mssql")]
    #[test]
    fn sql_server_gets_the_limit_it_understands() {
        use quaint::visitor::Visitor;

        let (sql, _) = quaint::visitor::Mssql::build(count_rows_query(Some("dbo"), "Cat", 1001)).unwrap();

        // However SQL Server spells it, the database stops reading at the limit.
        assert!(sql.contains("FETCH NEXT") || sql.contains("TOP"), "{sql}");
        assert!(sql.contains("COUNT(*)"), "{sql}");
    }
}
