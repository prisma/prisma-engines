//! API type definitions used by the JSON-RPC methods.

pub use crate::{
    js_result::JsResult,
    migration_directory::{MigrationDirectory, MigrationFile},
};
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use tsify_next::Tsify;

// ---- Common type definitions ----

/// Information about a migration lockfile.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct MigrationLockfile {
    /// Relative path to the lockfile from base directory.
    /// E.g., `./migration_lock.toml`.
    pub path: String,

    /// Content of the lockfile, if it exists.
    pub content: Option<String>,
}

/// A list of migration directories with related information.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct MigrationList {
    /// Absolute path to the base directory of Prisma migrations.
    /// E.g., `/usr/src/app/prisma/migrations`.
    pub base_dir: String,

    /// Description of the lockfile, which may or may not exist.
    pub lockfile: MigrationLockfile,

    /// List of migration directories.
    pub migration_directories: Vec<MigrationDirectory>,
}

/// An object with a `url` field.
/// @deprecated
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct UrlContainer {
    /// The URL string.
    pub url: String,
}

/// A container that holds the path and the content of a Prisma schema file.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct SchemaContainer {
    /// The content of the Prisma schema file.
    pub content: String,

    /// The file name of the Prisma schema file.
    pub path: String,
}

/// A container that holds multiple Prisma schema files.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct SchemasContainer {
    /// List of schema files.
    pub files: Vec<SchemaContainer>,
}

/// A list of Prisma schema files with a config directory.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SchemasWithConfigDir {
    /// A list of Prisma schema files.
    pub files: Vec<SchemaContainer>,

    /// An optional directory containing the config files such as SSL certificates.
    pub config_dir: String,
}

/// The path to a live database taken as input. For flexibility, this can be Prisma schemas as strings, or only the
/// connection string. See variants.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(tag = "tag")]
pub enum DatasourceParam {
    /// Prisma schema as input
    Schema(SchemasContainer),

    /// Connection string as input
    ConnectionString(UrlContainer),
}

/// A supported source for a database schema to diff in the `diff` command.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(tag = "tag", rename_all = "camelCase")]
pub enum DiffTarget {
    /// An empty schema.
    Empty,

    /// The Prisma schema content. The _datasource url_ will be considered, and the
    /// live database it points to introspected for its schema.
    SchemaDatasource(SchemasWithConfigDir),

    /// The Prisma schema content. The contents of the schema itself will be
    /// considered. This source does not need any database connection.
    SchemaDatamodel(SchemasContainer),

    /// The url to a live database. Its schema will be considered.
    ///
    /// This will cause the schema engine to connect to the database and read from it.
    /// It will not write.
    Url(UrlContainer),

    /// The Prisma schema content for migrations. The migrations will be applied to a **shadow database**, and the resulting schema
    /// considered for diffing.
    Migrations(MigrationList),
}

/// A diagnostic returned by `diagnoseMigrationHistory` when looking at the
/// database migration history in relation to the migrations directory.
#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(tag = "diagnostic", rename_all = "camelCase")]
pub enum HistoryDiagnostic {
    /// There are migrations in the migrations directory that have not been
    /// applied to the database yet.
    #[serde(rename_all = "camelCase")]
    DatabaseIsBehind {
        /// The names of the migrations.
        unapplied_migration_names: Vec<String>,
    },
    /// Migrations have been applied to the database that are not in the
    /// migrations directory.
    #[serde(rename_all = "camelCase")]
    MigrationsDirectoryIsBehind {
        /// The names of the migrations.
        unpersisted_migration_names: Vec<String>,
    },
    /// The migrations table history and the migrations directory history are
    /// not the same. This currently ignores the ordering of migrations.
    #[serde(rename_all = "camelCase")]
    HistoriesDiverge {
        /// The last migration that is present both in the migrations directory
        /// and the migrations table.
        last_common_migration_name: Option<String>,
        /// The names of the migrations that are present in the migrations table
        /// but not in the migrations directory.
        unpersisted_migration_names: Vec<String>,
        /// The names of the migrations that are present in the migrations
        /// directory but have not been applied to the database.
        unapplied_migration_names: Vec<String>,
    },
}

/// Fields for the DatabaseIsBehind variant.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct DatabaseIsBehindFields {}

/// The location of the live database to connect to.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(tag = "tag", rename_all = "camelCase")]
pub enum DbExecuteDatasourceType {
    /// Prisma schema files and content to take the datasource URL from.
    Schema(SchemasWithConfigDir),

    /// The URL of the database to run the command on.
    Url(UrlContainer),
}

/// A suggested action for the CLI `migrate dev` command.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(tag = "tag", rename_all = "camelCase")]
pub enum DevAction {
    /// Reset the database.
    Reset(DevActionReset),

    /// Proceed to the next step
    CreateMigration,
}

/// Reset action fields.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct DevActionReset {
    /// Why do we need to reset?
    pub reason: String,
}

// ---- JSON-RPC API types ----

// Apply Migrations

/// The input to the `applyMigrations` command.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationsInput {
    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,
}

/// The output of the `applyMigrations` command.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct ApplyMigrationsOutput {
    /// The names of the migrations that were just applied. Empty if no migration was applied.
    pub applied_migration_names: Vec<String>,
}

// Create Database

/// The type of params for the `createDatabase` method.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
pub struct CreateDatabaseParams {
    /// The datasource parameter.
    pub datasource: DatasourceParam,
}

/// The result for the `createDatabase` method.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct CreateDatabaseResult {
    /// The name of the created database.
    pub database_name: String,
}

// Create Migration

/// The input to the `createMigration` command.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct CreateMigrationInput {
    /// If true, always generate a migration, but do not apply.
    pub draft: bool,

    /// The user-given name for the migration. This will be used for the migration directory.
    pub migration_name: String,

    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,

    /// The Prisma schema content to use as a target for the generated migration.
    pub schema: SchemasContainer,
}

/// The output of the `createMigration` command.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct CreateMigrationOutput {
    /// The active connector type used.
    #[cfg_attr(target_arch = "wasm32", tsify(type = "'sqlite' | 'postgresql' | 'cockroachdb'"))]
    pub connector_type: String,

    /// The generated name of migration directory, which the caller must use to create the new directory.
    pub generated_migration_name: String,

    /// The migration script that was generated, if any.
    /// It will be null if:
    /// 1. The migration we generate would be empty, **AND**
    /// 2. the `draft` param was not true, because in that case the engine would still generate an empty
    ///    migration script.
    pub migration_script: Option<String>,

    /// The file extension for generated migration files.
    pub extension: String,
}

// DB Execute

/// The type of params accepted by dbExecute.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DbExecuteParams {
    /// The location of the live database to connect to.
    pub datasource_type: DbExecuteDatasourceType,

    /// The input script.
    pub script: String,
}

/// The type of results returned by dbExecute.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct DbExecuteResult {}

// Debug Panic

/// Request for debug panic.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct DebugPanicInput {}

/// Response for debug panic.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct DebugPanicOutput {}

// Dev Diagnostic

/// The request type for `devDiagnostic`.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DevDiagnosticInput {
    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,
}

/// The response type for `devDiagnostic`.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct DevDiagnosticOutput {
    /// The suggested course of action for the CLI.
    pub action: DevAction,
}

// Diagnose Migration History

/// The request params for the `diagnoseMigrationHistory` method.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryInput {
    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,

    /// Whether creating a shadow database is allowed.
    pub opt_in_to_shadow_database: bool,
}

/// The result type for `diagnoseMigrationHistory` responses.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DiagnoseMigrationHistoryOutput {
    /// The names of the migrations for which the checksum of the script in the
    /// migration directory does not match the checksum of the applied migration
    /// in the database.
    pub edited_migration_names: Vec<String>,

    /// The names of the migrations that are currently in a failed state in the migrations table.
    pub failed_migration_names: Vec<String>,

    /// Is the migrations table initialized/present in the database?
    pub has_migrations_table: bool,

    /// The current status of the migration history of the database
    /// relative to migrations directory. `null` if they are in sync and up
    /// to date.
    pub history: Option<HistoryDiagnostic>,
}

// Diff

/// The type of params for the `diff` method.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DiffParams {
    /// The source of the schema to consider as a _starting point_.
    pub from: DiffTarget,

    /// The source of the schema to consider as a _destination_, or the desired
    /// end-state.
    pub to: DiffTarget,

    /// The URL to a live database to use as a shadow database. The schema and data on
    /// that database will be wiped during diffing.
    ///
    /// This is only necessary when one of `from` or `to` is referencing a migrations
    /// directory as a source for the schema.
    /// @deprecated.
    pub shadow_database_url: Option<String>,

    /// By default, the response will contain a human-readable diff. If you want an
    /// executable script, pass the `"script": true` param.
    pub script: bool,

    /// Whether the --exit-code param was passed.
    ///
    /// If this is set, the engine will return exitCode = 2 in the diffResult in case the diff is
    /// non-empty. Other than this, it does not change the behaviour of the command.
    pub exit_code: Option<bool>,
}

/// The result type for the `diff` method.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    /// The exit code that the CLI should return.
    pub exit_code: u32,

    /// The diff script, if `script` was set to true in [`DiffParams`](DiffParams),
    /// or a human-readable migration summary otherwise.
    /// This is meant to be printed to the stdout by the caller.
    /// Note: in `schema-engine-cli`, this is None.
    pub stdout: Option<String>,
}

// Introspect SQL

/// Params type for the introspectSql method.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
pub struct IntrospectSqlParams {
    /// The database URL.
    pub url: String,
    /// SQL queries to introspect.
    pub queries: Vec<SqlQueryInput>,
}

/// Result type for the introspectSql method.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct IntrospectSqlResult {
    /// The introspected queries.
    pub queries: Vec<SqlQueryOutput>,
}

/// Input for a single SQL query.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
pub struct SqlQueryInput {
    /// The name of the query.
    pub name: String,
    /// The source SQL.
    pub source: String,
}

/// Output for a single SQL query.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SqlQueryOutput {
    /// The name of the query.
    pub name: String,
    /// The source SQL.
    pub source: String,
    /// Optional documentation.
    pub documentation: Option<String>,
    /// Query parameters.
    pub parameters: Vec<SqlQueryParameterOutput>,
    /// Query result columns.
    pub result_columns: Vec<SqlQueryColumnOutput>,
}

/// Information about a SQL query parameter.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct SqlQueryParameterOutput {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub typ: String,
    /// Optional documentation.
    pub documentation: Option<String>,
    /// Whether the parameter is nullable.
    pub nullable: bool,
}

/// Information about a SQL query result column.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct SqlQueryColumnOutput {
    /// Column name.
    pub name: String,
    /// Column type.
    pub typ: String,
    /// Whether the column is nullable.
    pub nullable: bool,
}

// Introspect

/// Introspect the database (db pull)
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct IntrospectParams {
    /// Prisma schema files.
    pub schema: SchemasContainer,
    /// Base directory path.
    pub base_directory_path: String,
    /// Force flag.
    pub force: bool,
    /// Composite type depth.
    pub composite_type_depth: isize,
    /// Optional namespaces.
    pub namespaces: Option<Vec<String>>,
}

/// Result type for the introspect method.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct IntrospectResult {
    /// The introspected schema.
    pub schema: SchemasContainer,
    /// Optional views.
    pub views: Option<Vec<IntrospectionView>>,
    /// Optional warnings.
    pub warnings: Option<String>,
}

/// Information about a database view.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct IntrospectionView {
    /// The view definition.
    pub definition: String,
    /// The view name.
    pub name: String,
    /// The schema name.
    pub schema: String,
}

// Get Database Version

/// Get the database version for error reporting.
/// @deprecated
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
pub struct GetDatabaseVersionInput {
    /// The datasource parameter.
    pub datasource: DatasourceParam,
}

// Evaluate Data Loss

/// Development command for migrations. Evaluate the data loss induced by the next
/// migration the engine would generate on the main database.
///
/// At this stage, the engine does not create or mutate anything in the database
/// nor in the migrations directory.
///
/// This is part of the `migrate dev` flow.
///
/// **Note**: the engine currently assumes the main database schema is up-to-date
/// with the migration history.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct EvaluateDataLossInput {
    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,
    /// The prisma schema files to migrate to.
    pub schema: SchemasContainer,
}

/// The output of the `evaluateDataLoss` command.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct EvaluateDataLossOutput {
    /// The number migration steps that would be generated. If this is empty, we
    /// wouldn't generate a new migration, unless the `draft` option is
    /// passed.
    pub migration_steps: u32,
    /// Steps that cannot be executed on the local database in the
    /// migration that would be generated.
    pub unexecutable_steps: Vec<MigrationFeedback>,
    /// Destructive change warnings for the local database. These are the
    /// warnings *for the migration that would be generated*. This does not
    /// include other potentially yet unapplied migrations.
    pub warnings: Vec<MigrationFeedback>,
}

/// A data loss warning or an unexecutable migration error, associated with the step that triggered it.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct MigrationFeedback {
    /// The human-readable message.
    pub message: String,
    /// The index of the step this pertains to.
    pub step_index: u32,
}

// Ensure Connection Validity

/// Make sure the schema engine can connect to the database from the Prisma schema.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
pub struct EnsureConnectionValidityParams {
    /// The datasource parameter.
    pub datasource: DatasourceParam,
}

/// Result type for the ensureConnectionValidity method.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct EnsureConnectionValidityResult {}

// Mark Migration Applied

/// Mark a migration as applied in the migrations table.
///
/// There are two possible outcomes:
///
/// - The migration is already in the table, but in a failed state. In this case, we will mark it
///   as rolled back, then create a new entry.
/// - The migration is not in the table. We will create a new entry in the migrations table. The
///   `started_at` and `finished_at` will be the same.
/// - If it is already applied, we return a user-facing error.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct MarkMigrationAppliedInput {
    /// The name of the migration to mark applied.
    pub migration_name: String,

    /// The list of migrations, already loaded from disk.
    pub migrations_list: MigrationList,
}

/// The output of the `markMigrationApplied` command.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct MarkMigrationAppliedOutput {}

// Mark Migration Rolled Back

/// Mark an existing failed migration as rolled back in the migrations table. It
/// will still be there, but ignored for all purposes except as audit trail.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct MarkMigrationRolledBackInput {
    /// The name of the migration to mark rolled back.
    pub migration_name: String,
}

/// The output of the `markMigrationRolledBack` command.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct MarkMigrationRolledBackOutput {}

// Reset

/// The input to the `reset` command.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
pub struct ResetInput {}

/// The output of the `reset` command.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
pub struct ResetOutput {}

// Schema Push

/// Request params for the `schemaPush` method.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, from_wasm_abi))]
pub struct SchemaPushInput {
    /// Push the schema ignoring destructive change warnings.
    pub force: bool,

    /// The Prisma schema files.
    pub schema: SchemasContainer,
}

/// Response result for the `schemaPush` method.
#[derive(Debug, Serialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(missing_as_null, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SchemaPushOutput {
    /// How many migration steps were executed.
    pub executed_steps: u32,

    /// Steps that cannot be executed in the current state of the database.
    pub unexecutable: Vec<String>,

    /// Destructive change warnings.
    pub warnings: Vec<String>,
}
