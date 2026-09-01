//! Replaying a migration history into a shadow database the user provided.
//!
//! Such a database is reset before the history is replayed into it, which destroys whatever it
//! holds, so it is only ever used when it holds nothing or when the user said that its contents
//! can go. It is also left empty afterwards, which is what makes the promise hold for the next
//! replay: a command that replays the history several times finds an empty database every time
//! after the first, and so does the command after it.

use crate::flavour::{SqlConnector, UsingExternalShadowDb};
use schema_connector::{ConnectorError, ConnectorResult, Namespaces, SchemaFilter, migrations_directory::Migrations};
use sql_schema_describer::SqlSchema;
use user_facing_errors::schema_engine::ShadowDbNotEmpty;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
