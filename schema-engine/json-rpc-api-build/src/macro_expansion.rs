// Recursive expansion of include! macro
// ======================================

pub mod json_rpc {
    #![doc = " The JSON-RPC API definition."]
    #![doc = ""]
    #![doc = " ## Methods"]
    #![doc = ""]
    #![doc = ""]
    #![doc = " ### 🔌 applyMigrations"]
    #![doc = " ➡️  [ApplyMigrationsInput](./types/struct.ApplyMigrationsInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [ApplyMigrationsOutput](./types/struct.ApplyMigrationsOutput.html)"]
    #![doc = ""]
    #![doc = " Apply the migrations from the migrations directory to the database."]
    #![doc = " "]
    #![doc = " This is the command behind `prisma migrate deploy`."]
    #![doc = ""]
    #![doc = " ### 🔌 createDatabase"]
    #![doc = " ➡️  [CreateDatabaseParams](./types/struct.CreateDatabaseParams.html)"]
    #![doc = ""]
    #![doc = " ↩️  [CreateDatabaseResult](./types/struct.CreateDatabaseResult.html)"]
    #![doc = ""]
    #![doc = " Create the logical database from the Prisma schema."]
    #![doc = ""]
    #![doc = " ### 🔌 createMigration"]
    #![doc = " ➡️  [CreateMigrationInput](./types/struct.CreateMigrationInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [CreateMigrationOutput](./types/struct.CreateMigrationOutput.html)"]
    #![doc = ""]
    #![doc = " Create the next migration in the migrations history. If `draft` is false and"]
    #![doc = " there are no unexecutable steps, it will also apply the newly created"]
    #![doc = " migration."]
    #![doc = " "]
    #![doc = " **Note**: This will use the shadow database on the connectors where we need"]
    #![doc = " one."]
    #![doc = ""]
    #![doc = " ### 🔌 dbExecute"]
    #![doc = " ➡️  [DbExecuteParams](./types/struct.DbExecuteParams.html)"]
    #![doc = ""]
    #![doc = " ↩️  [DbExecuteResult](./types/struct.DbExecuteResult.html)"]
    #![doc = ""]
    #![doc = " Execute a database script directly on the specified live database."]
    #![doc = " "]
    #![doc = " Note that this may not be"]
    #![doc = " defined on all connectors."]
    #![doc = ""]
    #![doc = " ### 🔌 debugPanic"]
    #![doc = " ➡️  [DebugPanicInput](./types/struct.DebugPanicInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [DebugPanicOutput](./types/struct.DebugPanicOutput.html)"]
    #![doc = ""]
    #![doc = " Make the schema engine panic. Only useful to test client error handling."]
    #![doc = ""]
    #![doc = " ### 🔌 devDiagnostic"]
    #![doc = " ➡️  [DevDiagnosticInput](./types/struct.DevDiagnosticInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [DevDiagnosticOutput](./types/struct.DevDiagnosticOutput.html)"]
    #![doc = ""]
    #![doc = " The method called at the beginning of `migrate dev` to decide the course of"]
    #![doc = " action based on the current state of the workspace."]
    #![doc = " "]
    #![doc = " It acts as a wrapper around diagnoseMigrationHistory. Its role is to interpret"]
    #![doc = " the diagnostic output, and translate it to a concrete action to be performed by"]
    #![doc = " the CLI."]
    #![doc = ""]
    #![doc = " ### 🔌 diagnoseMigrationHistory"]
    #![doc = " ➡️  [DiagnoseMigrationHistoryInput](./types/struct.DiagnoseMigrationHistoryInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [DiagnoseMigrationHistoryOutput](./types/struct.DiagnoseMigrationHistoryOutput.html)"]
    #![doc = ""]
    #![doc = " Read the contents of the migrations directory and the migrations table,"]
    #![doc = " and returns their relative statuses. At this stage, the migration"]
    #![doc = " engine only reads, it does not write to the database nor the migrations"]
    #![doc = " directory, nor does it use a shadow database."]
    #![doc = ""]
    #![doc = " ### 🔌 diff"]
    #![doc = " ➡️  [DiffParams](./types/struct.DiffParams.html)"]
    #![doc = ""]
    #![doc = " ↩️  [DiffResult](./types/struct.DiffResult.html)"]
    #![doc = ""]
    #![doc = " Compares two databases schemas from two arbitrary sources, and display the"]
    #![doc = " difference as either a human-readable summary, or an executable script that can"]
    #![doc = " be passed to dbExecute."]
    #![doc = " "]
    #![doc = " Connection to a shadow database is only necessary when either the `from` or the"]
    #![doc = " `to` params is a migrations directory."]
    #![doc = " "]
    #![doc = " Diffs have a _direction_. Which source is `from` and which is `to` matters. The"]
    #![doc = " resulting diff should be thought as a migration from the schema in `from` to"]
    #![doc = " the schema in `to`."]
    #![doc = " "]
    #![doc = " By default, we output a human-readable diff. If you want an executable script,"]
    #![doc = " pass the `\"script\": true` param."]
    #![doc = ""]
    #![doc = " ### 🔌 ensureConnectionValidity"]
    #![doc = " ➡️  [EnsureConnectionValidityParams](./types/struct.EnsureConnectionValidityParams.html)"]
    #![doc = ""]
    #![doc = " ↩️  [EnsureConnectionValidityResult](./types/struct.EnsureConnectionValidityResult.html)"]
    #![doc = ""]
    #![doc = " Make sure the schema engine can connect to the database from the Prisma schema."]
    #![doc = ""]
    #![doc = " ### 🔌 evaluateDataLoss"]
    #![doc = " ➡️  [EvaluateDataLossInput](./types/struct.EvaluateDataLossInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [EvaluateDataLossOutput](./types/struct.EvaluateDataLossOutput.html)"]
    #![doc = ""]
    #![doc = " Development command for migrations. Evaluate the data loss induced by the next"]
    #![doc = " migration the engine would generate on the main database."]
    #![doc = " "]
    #![doc = " At this stage, the engine does not create or mutate anything in the database"]
    #![doc = " nor in the migrations directory."]
    #![doc = " "]
    #![doc = " This is part of the `migrate dev` flow."]
    #![doc = " "]
    #![doc = " **Note**: the engine currently assumes the main database schema is up-to-date"]
    #![doc = " with the migration history."]
    #![doc = ""]
    #![doc = " ### 🔌 getDatabaseVersion"]
    #![doc = " ➡️  [GetDatabaseVersionInput](./types/struct.GetDatabaseVersionInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [GetDatabaseVersionOutput](./types/struct.GetDatabaseVersionOutput.html)"]
    #![doc = ""]
    #![doc = " Get the database version for error reporting."]
    #![doc = ""]
    #![doc = " ### 🔌 introspect"]
    #![doc = " ➡️  [IntrospectParams](./types/struct.IntrospectParams.html)"]
    #![doc = ""]
    #![doc = " ↩️  [IntrospectResult](./types/struct.IntrospectResult.html)"]
    #![doc = ""]
    #![doc = " Introspect the database (db pull)"]
    #![doc = ""]
    #![doc = " ### 🔌 introspectSql"]
    #![doc = " ➡️  [IntrospectSqlParams](./types/struct.IntrospectSqlParams.html)"]
    #![doc = ""]
    #![doc = " ↩️  [IntrospectSqlResult](./types/struct.IntrospectSqlResult.html)"]
    #![doc = ""]
    #![doc = " Introspect a SQL query and returns type information"]
    #![doc = ""]
    #![doc = " ### 🔌 listMigrationDirectories"]
    #![doc = " ➡️  [ListMigrationDirectoriesInput](./types/struct.ListMigrationDirectoriesInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [ListMigrationDirectoriesOutput](./types/struct.ListMigrationDirectoriesOutput.html)"]
    #![doc = ""]
    #![doc = " List the names of the migrations in the migrations directory."]
    #![doc = ""]
    #![doc = " ### 🔌 markMigrationApplied"]
    #![doc = " ➡️  [MarkMigrationAppliedInput](./types/struct.MarkMigrationAppliedInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [MarkMigrationAppliedOutput](./types/struct.MarkMigrationAppliedOutput.html)"]
    #![doc = ""]
    #![doc = " Mark a migration as applied in the migrations table."]
    #![doc = " "]
    #![doc = " There are two possible outcomes:"]
    #![doc = " "]
    #![doc = " - The migration is already in the table, but in a failed state. In this case, we will mark it"]
    #![doc = "   as rolled back, then create a new entry."]
    #![doc = " - The migration is not in the table. We will create a new entry in the migrations table. The"]
    #![doc = "   `started_at` and `finished_at` will be the same."]
    #![doc = " - If it is already applied, we return a user-facing error."]
    #![doc = ""]
    #![doc = " ### 🔌 markMigrationRolledBack"]
    #![doc = " ➡️  [MarkMigrationRolledBackInput](./types/struct.MarkMigrationRolledBackInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [MarkMigrationRolledBackOutput](./types/struct.MarkMigrationRolledBackOutput.html)"]
    #![doc = ""]
    #![doc = " Mark an existing failed migration as rolled back in the migrations table. It"]
    #![doc = " will still be there, but ignored for all purposes except as audit trail."]
    #![doc = ""]
    #![doc = " ### 🔌 reset"]
    #![doc = " ➡️  [ResetInput](./types/struct.ResetInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [ResetOutput](./types/struct.ResetOutput.html)"]
    #![doc = ""]
    #![doc = " Try to make the database empty: no data and no schema. On most connectors, this"]
    #![doc = " is implemented by dropping and recreating the database. If that fails (most"]
    #![doc = " likely because of insufficient permissions), the engine attemps a \"best effort"]
    #![doc = " reset\" by inspecting the contents of the database and dropping them"]
    #![doc = " individually."]
    #![doc = " "]
    #![doc = " Drop and recreate the database. The migrations will not be applied, as it would"]
    #![doc = " overlap with `applyMigrations`."]
    #![doc = ""]
    #![doc = " ### 🔌 schemaPush"]
    #![doc = " ➡️  [SchemaPushInput](./types/struct.SchemaPushInput.html)"]
    #![doc = ""]
    #![doc = " ↩️  [SchemaPushOutput](./types/struct.SchemaPushOutput.html)"]
    #![doc = ""]
    #![doc = " The command behind `db push`."]
    #[doc = " String constants for method names."]
    pub mod method_names {
        #[doc = " Exhaustive list of the names of all JSON-RPC methods."]
        pub const METHOD_NAMES: &[&str] = &[
            "applyMigrations",
            "createDatabase",
            "createMigration",
            "dbExecute",
            "debugPanic",
            "devDiagnostic",
            "diagnoseMigrationHistory",
            "diff",
            "ensureConnectionValidity",
            "evaluateDataLoss",
            "getDatabaseVersion",
            "introspect",
            "introspectSql",
            "listMigrationDirectories",
            "markMigrationApplied",
            "markMigrationRolledBack",
            "reset",
            "schemaPush",
        ];
        #[doc = " applyMigrations"]
        pub const APPLY_MIGRATIONS: &str = "applyMigrations";
        #[doc = " createDatabase"]
        pub const CREATE_DATABASE: &str = "createDatabase";
        #[doc = " createMigration"]
        pub const CREATE_MIGRATION: &str = "createMigration";
        #[doc = " dbExecute"]
        pub const DB_EXECUTE: &str = "dbExecute";
        #[doc = " debugPanic"]
        pub const DEBUG_PANIC: &str = "debugPanic";
        #[doc = " devDiagnostic"]
        pub const DEV_DIAGNOSTIC: &str = "devDiagnostic";
        #[doc = " diagnoseMigrationHistory"]
        pub const DIAGNOSE_MIGRATION_HISTORY: &str = "diagnoseMigrationHistory";
        #[doc = " diff"]
        pub const DIFF: &str = "diff";
        #[doc = " ensureConnectionValidity"]
        pub const ENSURE_CONNECTION_VALIDITY: &str = "ensureConnectionValidity";
        #[doc = " evaluateDataLoss"]
        pub const EVALUATE_DATA_LOSS: &str = "evaluateDataLoss";
        #[doc = " getDatabaseVersion"]
        pub const GET_DATABASE_VERSION: &str = "getDatabaseVersion";
        #[doc = " introspect"]
        pub const INTROSPECT: &str = "introspect";
        #[doc = " introspectSql"]
        pub const INTROSPECT_SQL: &str = "introspectSql";
        #[doc = " listMigrationDirectories"]
        pub const LIST_MIGRATION_DIRECTORIES: &str = "listMigrationDirectories";
        #[doc = " markMigrationApplied"]
        pub const MARK_MIGRATION_APPLIED: &str = "markMigrationApplied";
        #[doc = " markMigrationRolledBack"]
        pub const MARK_MIGRATION_ROLLED_BACK: &str = "markMigrationRolledBack";
        #[doc = " reset"]
        pub const RESET: &str = "reset";
        #[doc = " schemaPush"]
        pub const SCHEMA_PUSH: &str = "schemaPush";
    }
    #[doc = " API type definitions used by the methods."]
    #[allow(missing_docs)]
    pub mod types {
        use serde::{Deserialize, Serialize};
        #[derive(Serialize, Deserialize, Debug)]
        pub struct GetDatabaseVersionInput {
            pub datasource: DatasourceParam,
        }
        #[doc = " The request type for `devDiagnostic`."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DevDiagnosticInput {
            #[doc = " The location of the migrations directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DebugPanicOutput {}

        #[derive(Serialize, Deserialize, Debug)]
        pub struct DevActionReset {
            #[doc = " Why do we need to reset?"]
            pub reason: String,
        }
        #[doc = " The output of the `evaluateDataLoss` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct EvaluateDataLossOutput {
            #[doc = " The number migration steps that would be generated. If this is empty, we"]
            #[doc = " wouldn\'t generate a new migration, unless the `draft` option is"]
            #[doc = " passed."]
            #[doc = ""]
            #[doc = " JSON name: migrationSteps"]
            #[serde(rename = "migrationSteps")]
            pub migration_steps: u32,
            #[doc = " Steps that cannot be executed on the local database in the"]
            #[doc = " migration that would be generated."]
            #[doc = ""]
            #[doc = " JSON name: unexecutableSteps"]
            #[serde(rename = "unexecutableSteps")]
            pub unexecutable_steps: Vec<MigrationFeedback>,
            #[doc = " Destructive change warnings for the local database. These are the"]
            #[doc = " warnings *for the migration that would be generated*. This does not"]
            #[doc = " include other potentially yet unapplied migrations."]
            pub warnings: Vec<MigrationFeedback>,
        }
        #[doc = " The type of params for the `diff` method."]
        #[doc = " ### Example"]
        #[doc = ""]
        #[doc = " ```ignore"]
        #[doc = " {"]
        #[doc = "     \"from\": {"]
        #[doc = "         \"tag\": \"migrations\","]
        #[doc = "         \"path\": \"./prisma/migrations\""]
        #[doc = "     },"]
        #[doc = "     \"to\": {"]
        #[doc = "         \"tag\": \"schemaDatamodel\","]
        #[doc = "         \"schema\": \"./prisma/schema.prisma\","]
        #[doc = "     }"]
        #[doc = "     \"shadowDatabaseUrl\": \"mysql://test/test\""]
        #[doc = " }"]
        #[doc = " ```"]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DiffParams {
            #[doc = " Whether the --exit-code param was passed."]
            #[doc = " "]
            #[doc = " If this is set, the engine will return exitCode = 2 in the diffResult in case the diff is"]
            #[doc = " non-empty. Other than this, it does not change the behaviour of the command."]
            #[doc = ""]
            #[doc = " JSON name: exitCode"]
            #[serde(rename = "exitCode")]
            pub exit_code: Option<bool>,
            #[doc = " The source of the schema to consider as a _starting point_."]
            pub from: DiffTarget,
            #[doc = " By default, the response will contain a human-readable diff. If you want an"]
            #[doc = " executable script, pass the `\"script\": true` param."]
            pub script: bool,
            #[doc = " The URL to a live database to use as a shadow database. The schema and data on"]
            #[doc = " that database will be wiped during diffing."]
            #[doc = " "]
            #[doc = " This is only necessary when one of `from` or `to` is referencing a migrations"]
            #[doc = " directory as a source for the schema."]
            #[doc = ""]
            #[doc = " JSON name: shadowDatabaseUrl"]
            #[serde(rename = "shadowDatabaseUrl")]
            pub shadow_database_url: Option<String>,
            #[doc = " The source of the schema to consider as a _destination_, or the desired"]
            #[doc = " end-state."]
            pub to: DiffTarget,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SqlQueryColumnOutput {
            pub name: String,
            pub nullable: bool,
            pub typ: String,
        }
        #[doc = " The response type for `devDiagnostic`."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DevDiagnosticOutput {
            #[doc = " The suggested course of action for the CLI."]
            pub action: DevAction,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct CreateDatabaseParams {
            pub datasource: DatasourceParam,
        }
        #[doc = " A data loss warning or an unexecutable migration error, associated with the step that triggered it."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct MigrationFeedback {
            #[doc = " The human-readable message."]
            pub message: String,
            #[doc = " The index of the step this pertains to."]
            #[doc = ""]
            #[doc = " JSON name: stepIndex"]
            #[serde(rename = "stepIndex")]
            pub step_index: u32,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct ListMigrationDirectoriesOutput {
            #[doc = " The names of the migrations in the migration directory. Empty if no migrations are found."]
            pub migrations: Vec<String>,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct EnsureConnectionValidityParams {
            pub datasource: DatasourceParam,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DebugPanicInput {}

        #[doc = " Params type for the introspectSql method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct IntrospectSqlParams {
            pub queries: Vec<SqlQueryInput>,
            pub url: String,
        }
        #[doc = " Result type for the introspect method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct IntrospectResult {
            pub schema: SchemasContainer,
            pub views: Option<Vec<IntrospectionView>>,
            pub warnings: Option<String>,
        }
        #[doc = " A container that holds the path and the content of a Prisma schema file."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SchemaContainer {
            #[doc = " The content of the Prisma schema file."]
            pub content: String,
            #[doc = " The file name of the Prisma schema file."]
            pub path: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct ResetInput {}

        #[doc = " The input to the `listMigrationDirectories` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct ListMigrationDirectoriesInput {
            #[doc = " The location of the migrations directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct EnsureConnectionValidityResult {}

        #[derive(Serialize, Deserialize, Debug)]
        pub struct CreateDatabaseResult {
            #[doc = ""]
            #[doc = " JSON name: databaseName"]
            #[serde(rename = "databaseName")]
            pub database_name: String,
        }
        #[doc = " Request params for the `schemaPush` method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SchemaPushInput {
            #[doc = " Push the schema ignoring destructive change warnings."]
            pub force: bool,
            #[doc = " The Prisma schema files."]
            pub schema: SchemasContainer,
        }
        #[doc = " An object with a `url` field."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct UrlContainer {
            pub url: String,
        }
        #[doc = " The names of the migrations in the migration directory. Empty if no migrations are found."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct MarkMigrationAppliedInput {
            #[doc = " The name of the migration to mark applied."]
            #[doc = ""]
            #[doc = " JSON name: migrationName"]
            #[serde(rename = "migrationName")]
            pub migration_name: String,
            #[doc = " The path to the root of the migrations directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
        }
        #[doc = " The output of the `markMigrationRolledBack` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct MarkMigrationRolledBackOutput {}

        #[derive(Serialize, Deserialize, Debug)]
        pub struct SqlQueryParameterOutput {
            pub documentation: Option<String>,
            pub name: String,
            pub nullable: bool,
            pub typ: String,
        }
        #[doc = " Response result for the `schemaPush` method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SchemaPushOutput {
            #[doc = " How many migration steps were executed."]
            #[doc = ""]
            #[doc = " JSON name: executedSteps"]
            #[serde(rename = "executedSteps")]
            pub executed_steps: u32,
            #[doc = " Steps that cannot be executed in the current state of the database."]
            pub unexecutable: Vec<String>,
            #[doc = " Destructive change warnings."]
            pub warnings: Vec<String>,
        }
        #[doc = " The result type for the `diff` method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DiffResult {
            #[doc = " The exit code that the CLI should return."]
            #[doc = ""]
            #[doc = " JSON name: exitCode"]
            #[serde(rename = "exitCode")]
            pub exit_code: u32,
        }
        #[doc = " The input to the `applyMigrations` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct ApplyMigrationsInput {
            #[doc = " The location of the migrations directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct GetDatabaseVersionOutput {
            pub version: String,
        }
        #[doc = " The input to the `createMigration` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct CreateMigrationInput {
            #[doc = " If true, always generate a migration, but do not apply."]
            pub draft: bool,
            #[doc = " The user-given name for the migration. This will be used for the migration directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationName"]
            #[serde(rename = "migrationName")]
            pub migration_name: String,
            #[doc = " The filesystem path of the migrations directory to use."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
            #[doc = " The Prisma schema files to use as a target for the generated migration."]
            pub schema: SchemasContainer,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct PathContainer {
            pub path: String,
        }
        #[doc = " A list of Prisma schema files with a config directory."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SchemasWithConfigDir {
            #[doc = " An optional directory containing the config files such as SSL certificates."]
            #[doc = ""]
            #[doc = " JSON name: configDir"]
            #[serde(rename = "configDir")]
            pub config_dir: String,
            #[doc = " A list of Prisma schema files."]
            pub files: Vec<SchemaContainer>,
        }
        #[doc = " The type of results returned by dbExecute."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DbExecuteResult {}

        #[doc = " A container that holds multiple Prisma schema files."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SchemasContainer {
            pub files: Vec<SchemaContainer>,
        }
        #[doc = " The request params for the `diagnoseMigrationHistory` method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DiagnoseMigrationHistoryInput {
            #[doc = " The path to the root of the migrations directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
            #[doc = " Whether creating a shadow database is allowed."]
            #[doc = ""]
            #[doc = " JSON name: optInToShadowDatabase"]
            #[serde(rename = "optInToShadowDatabase")]
            pub opt_in_to_shadow_database: bool,
        }
        #[doc = " The result type for `diagnoseMigrationHistory` responses."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DiagnoseMigrationHistoryOutput {
            #[doc = " The names of the migrations for which the checksum of the script in the"]
            #[doc = " migration directory does not match the checksum of the applied migration"]
            #[doc = " in the database."]
            #[doc = ""]
            #[doc = " JSON name: editedMigrationNames"]
            #[serde(rename = "editedMigrationNames")]
            pub edited_migration_names: Vec<String>,
            #[doc = " The names of the migrations that are currently in a failed state in the migrations table."]
            #[doc = ""]
            #[doc = " JSON name: failedMigrationNames"]
            #[serde(rename = "failedMigrationNames")]
            pub failed_migration_names: Vec<String>,
            #[doc = " Is the migrations table initialized/present in the database?"]
            #[doc = ""]
            #[doc = " JSON name: hasMigrationsTable"]
            #[serde(rename = "hasMigrationsTable")]
            pub has_migrations_table: bool,
            #[doc = " The current status of the migration history of the database"]
            #[doc = " relative to migrations directory. `null` if they are in sync and up"]
            #[doc = " to date."]
            pub history: Option<HistoryDiagnostic>,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DatabaseIsBehindFields {}

        #[derive(Serialize, Deserialize, Debug)]
        pub struct ResetOutput {}

        #[doc = " Params type for the introspect method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct IntrospectParams {
            #[doc = ""]
            #[doc = " JSON name: baseDirectoryPath"]
            #[serde(rename = "baseDirectoryPath")]
            pub base_directory_path: String,
            #[doc = ""]
            #[doc = " JSON name: compositeTypeDepth"]
            #[serde(rename = "compositeTypeDepth")]
            pub composite_type_depth: isize,
            pub force: bool,
            pub namespaces: Option<Vec<String>>,
            pub schema: SchemasContainer,
        }
        #[doc = " The output of the `creatMigration` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct CreateMigrationOutput {
            #[doc = " The name of the newly generated migration directory, if any."]
            #[doc = " "]
            #[doc = " generatedMigrationName will be null if: "]
            #[doc = " "]
            #[doc = " 1. The migration we generate would be empty, **AND**"]
            #[doc = " 2. the `draft` param was not true, because in that case the engine would still generate an empty"]
            #[doc = "    migration script."]
            #[doc = ""]
            #[doc = " JSON name: generatedMigrationName"]
            #[serde(rename = "generatedMigrationName")]
            pub generated_migration_name: Option<String>,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SqlQueryInput {
            pub name: String,
            pub source: String,
        }
        #[doc = " The output of the `markMigrationApplied` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct MarkMigrationAppliedOutput {}

        #[doc = " Result type for the introspectSql method."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct IntrospectSqlResult {
            pub queries: Vec<SqlQueryOutput>,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct SqlQueryOutput {
            pub documentation: Option<String>,
            pub name: String,
            pub parameters: Vec<SqlQueryParameterOutput>,
            #[doc = ""]
            #[doc = " JSON name: resultColumns"]
            #[serde(rename = "resultColumns")]
            pub result_columns: Vec<SqlQueryColumnOutput>,
            pub source: String,
        }
        #[doc = " The input to the `evaluateDataLoss` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct EvaluateDataLossInput {
            #[doc = " The location of the migrations directory."]
            #[doc = ""]
            #[doc = " JSON name: migrationsDirectoryPath"]
            #[serde(rename = "migrationsDirectoryPath")]
            pub migrations_directory_path: String,
            #[doc = " The prisma schema files to migrate to."]
            pub schema: SchemasContainer,
        }
        #[doc = " The output of the `applyMigrations` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct ApplyMigrationsOutput {
            #[doc = " The names of the migrations that were just applied. Empty if no migration was applied."]
            #[doc = ""]
            #[doc = " JSON name: appliedMigrationNames"]
            #[serde(rename = "appliedMigrationNames")]
            pub applied_migration_names: Vec<String>,
        }
        #[doc = " The type of params accepted by dbExecute."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct DbExecuteParams {
            #[doc = " The location of the live database to connect to."]
            #[doc = ""]
            #[doc = " JSON name: datasourceType"]
            #[serde(rename = "datasourceType")]
            pub datasource_type: DbExecuteDatasourceType,
            #[doc = " The input script."]
            pub script: String,
        }
        #[derive(Serialize, Deserialize, Debug)]
        pub struct IntrospectionView {
            pub definition: String,
            pub name: String,
            pub schema: String,
        }
        #[doc = " The input to the `markMigrationRolledBack` command."]
        #[derive(Serialize, Deserialize, Debug)]
        pub struct MarkMigrationRolledBackInput {
            #[doc = " The name of the migration to mark rolled back."]
            #[doc = ""]
            #[doc = " JSON name: migrationName"]
            #[serde(rename = "migrationName")]
            pub migration_name: String,
        }
        #[doc = " A diagnostic returned by `diagnoseMigrationHistory` when looking at the"]
        #[doc = " database migration history in relation to the migrations directory."]
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "tag")]
        pub enum HistoryDiagnostic {
            MigrationsDirectoryIsBehind,
            #[doc = " There are migrations in the migrations directory that have not been applied to"]
            #[doc = " the database yet."]
            DatabaseIsBehind(DatabaseIsBehindFields),
            HistoriesDiverge,
        }
        #[doc = " A supported source for a database schema to diff in the `diff` command."]
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "tag")]
        pub enum DiffTarget {
            #[doc = " The path to a migrations directory of the shape expected by Prisma Migrate. The"]
            #[doc = " migrations will be applied to a **shadow database**, and the resulting schema"]
            #[doc = " considered for diffing."]
            #[doc = ""]
            #[doc = " JSON name: migrations"]
            #[serde(rename = "migrations")]
            Migrations(PathContainer),
            #[doc = " The url to a live database. Its schema will be considered."]
            #[doc = " "]
            #[doc = " This will cause the schema engine to connect to the database and read from it."]
            #[doc = " It will not write."]
            #[doc = ""]
            #[doc = " JSON name: url"]
            #[serde(rename = "url")]
            Url(UrlContainer),
            #[doc = " The path to a Prisma schema. The contents of the schema itself will be"]
            #[doc = " considered. This source does not need any database connection."]
            #[doc = ""]
            #[doc = " JSON name: schemaDatamodel"]
            #[serde(rename = "schemaDatamodel")]
            SchemaDatamodel(SchemasContainer),
            #[doc = " An empty schema."]
            #[doc = ""]
            #[doc = " JSON name: empty"]
            #[serde(rename = "empty")]
            Empty,
            #[doc = " The path to a Prisma schema. The _datasource url_ will be considered, and the"]
            #[doc = " live database it points to introspected for its schema."]
            #[doc = ""]
            #[doc = " JSON name: schemaDatasource"]
            #[serde(rename = "schemaDatasource")]
            SchemaDatasource(SchemasWithConfigDir),
        }
        #[doc = " A suggested action for the CLI `migrate dev` command."]
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "tag")]
        pub enum DevAction {
            #[doc = " Proceed to the next step"]
            #[doc = ""]
            #[doc = " JSON name: createMigration"]
            #[serde(rename = "createMigration")]
            CreateMigration,
            #[doc = " Reset the database."]
            #[doc = ""]
            #[doc = " JSON name: reset"]
            #[serde(rename = "reset")]
            Reset(DevActionReset),
        }
        #[doc = " The location of the live database to connect to."]
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "tag")]
        pub enum DbExecuteDatasourceType {
            #[doc = " Prisma schema files and content to take the datasource URL from."]
            #[doc = ""]
            #[doc = " JSON name: schema"]
            #[serde(rename = "schema")]
            Schema(SchemasWithConfigDir),
            #[doc = " The URL of the database to run the command on."]
            #[doc = ""]
            #[doc = " JSON name: url"]
            #[serde(rename = "url")]
            Url(UrlContainer),
        }
        #[doc = " The path to a live database taken as input. For flexibility, this can be Prisma schemas as strings, or only the"]
        #[doc = " connection string. See variants."]
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(tag = "tag")]
        pub enum DatasourceParam {
            Schema(SchemasContainer),
            ConnectionString(UrlContainer),
        }
    }
}
