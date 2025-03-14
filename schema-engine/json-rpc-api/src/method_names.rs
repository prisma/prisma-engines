//! JSON-RPC API methods.

pub const APPLY_MIGRATIONS: &str = "applyMigrations";
pub const CREATE_DATABASE: &str = "createDatabase";
pub const CREATE_MIGRATION: &str = "createMigration";
pub const DB_EXECUTE: &str = "dbExecute";
pub const DEBUG_PANIC: &str = "debugPanic";
pub const DEV_DIAGNOSTIC: &str = "devDiagnostic";
pub const DIAGNOSE_MIGRATION_HISTORY: &str = "diagnoseMigrationHistory";
pub const DIFF: &str = "diff";
pub const ENSURE_CONNECTION_VALIDITY: &str = "ensureConnectionValidity";
pub const EVALUATE_DATA_LOSS: &str = "evaluateDataLoss";
pub const GET_DATABASE_VERSION: &str = "getDatabaseVersion";
pub const INTROSPECT: &str = "introspect";
pub const INTROSPECT_SQL: &str = "introspectSql";
pub const MARK_MIGRATION_APPLIED: &str = "markMigrationApplied";
pub const MARK_MIGRATION_ROLLED_BACK: &str = "markMigrationRolledBack";
pub const RESET: &str = "reset";
pub const SCHEMA_PUSH: &str = "schemaPush";

/// Exhaustive list of the names of all JSON-RPC methods.
pub const METHOD_NAMES: &[&str] = &[
    APPLY_MIGRATIONS,
    CREATE_DATABASE,
    CREATE_MIGRATION,
    DB_EXECUTE,
    DEBUG_PANIC,
    DEV_DIAGNOSTIC,
    DIAGNOSE_MIGRATION_HISTORY,
    DIFF,
    ENSURE_CONNECTION_VALIDITY,
    EVALUATE_DATA_LOSS,
    GET_DATABASE_VERSION,
    INTROSPECT,
    INTROSPECT_SQL,
    MARK_MIGRATION_APPLIED,
    MARK_MIGRATION_ROLLED_BACK,
    RESET,
    SCHEMA_PUSH,
];
