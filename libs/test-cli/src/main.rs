use anyhow::Context;
use colored::Colorize;
use migration_connector::ImperativeMigration;
use migration_core::commands::{GenerateImperativeMigrationInput, PushSchemaInput};
use std::{fs::File, io::Read};
use structopt::*;

#[derive(StructOpt)]
#[structopt(version = env!("GIT_HASH"))]
enum Command {
    /// Apply a prisma schema to a database
    ApplySchema {
        /// The path to the prisma schema file. Either this or --stdin should be provided.
        #[structopt(long)]
        file_path: Option<String>,
        /// Try to read the prisma schema from stdin. Either this or --file-path should be provided.
        #[structopt(long)]
        stdin: bool,
        /// Whether to ignore warnings from the migration engine regarding data loss. Default: false.
        #[structopt(long)]
        force: Option<bool>,
    },
    /// Introspect a database
    Introspect {
        /// URL of the database to introspect.
        #[structopt(long)]
        url: Option<String>,
        /// Path to the schema file to introspect for.
        #[structopt(long = "file-path")]
        file_path: Option<String>,
    },
    /// Generate DMMF from a schema, or directly from a database URl.
    Dmmf(DmmfCommand),
    /// Push a prisma schema directly to the database, without interacting with migrations.
    PushSchema(PushSchema),
    /// Save an imperative migration based on the provided schema and migrations folder.
    MigrateSave(MigrateSave),
}

#[derive(Debug, StructOpt)]
struct MigrateSave {
    #[structopt(long)]
    schema_path: String,
    #[structopt(long)]
    migrations_folder_path: String,
    #[structopt(long)]
    migration_name: String,
}

#[derive(StructOpt)]
struct PushSchema {
    schema_path: String,
    #[structopt(long)]
    force: bool,
}

#[derive(StructOpt)]
struct DmmfCommand {
    /// The path to the `query-engine` binary. Defaults to the value of the `PRISMA_BINARY_PATH`
    /// env var, or just `query-engine`.
    #[structopt(env = "PRISMA_BINARY_PATH", default_value = "query-engine")]
    query_engine_binary_path: String,
    /// A database URL to introspect and generate DMMF for.
    #[structopt(long = "url")]
    url: Option<String>,
    /// Path of the prisma schema to generate DMMF for.
    #[structopt(long = "file-path")]
    file_path: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    match Command::from_args() {
        Command::MigrateSave(cmd) => generate_migration(&cmd).await?,
        Command::PushSchema(cmd) => push_schema(&cmd).await?,
        Command::Dmmf(cmd) => generate_dmmf(&cmd).await?,
        Command::Introspect { url, file_path } => {
            if url.as_ref().xor(file_path.as_ref()).is_none() {
                anyhow::bail!(
                    "{}",
                    "Exactly one of --url or --file-path must be provided".bold().red()
                );
            }

            let schema = if let Some(file_path) = file_path {
                read_datamodel_from_file(&file_path)?
            } else if let Some(url) = url {
                minimal_schema_from_url(&url)?
            } else {
                unreachable!()
            };
            //todo configurable
            let introspected = introspection_core::RpcImpl::introspect_internal(schema, false)
                .await
                .map_err(|err| anyhow::anyhow!("{:?}", err.data))?;

            println!("{}", introspected);
        }
        Command::ApplySchema {
            file_path,
            force,
            stdin,
        } => {
            let datamodel_string: String = match (file_path, stdin) {
                (Some(path), false) => read_datamodel_from_file(&path).context("error reading the schemafile")?,
                (None, true) => read_datamodel_from_stdin()?,
                (Some(_), true) => {
                    anyhow::bail!("{}", "please pass either --stdin or --file-path, not both".bold().red())
                }
                (None, false) => anyhow::bail!("{}", "either --stdin or --file-path is required".bold().red()),
            };

            let api = migration_core::migration_api(&datamodel_string).await?;

            let migration_id = "test-cli-migration".to_owned();

            let infer_input = migration_core::InferMigrationStepsInput {
                assume_applied_migrations: Some(Vec::new()),
                assume_to_be_applied: Some(Vec::new()),
                datamodel: datamodel_string.clone(),
                migration_id: migration_id.clone(),
            };

            api.reset(&serde_json::Value::Null).await?;

            let result = api.infer_migration_steps(&infer_input).await?;

            let apply_input = migration_core::ApplyMigrationInput {
                force,
                migration_id,
                steps: result.datamodel_steps,
            };

            let result = api.apply_migration(&apply_input).await?;
            let warnings: Vec<_> = result.warnings.into_iter().map(|warning| warning.description).collect();

            if warnings.is_empty() {
                eprintln!("{}", "✔️  migrated without warning".bold().green());
            } else {
                for warning in warnings {
                    eprintln!("{} - {}", "⚠️ MIGRATION WARNING ⚠️ ".bold().red(), warning)
                }

                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn read_datamodel_from_file(path: &str) -> std::io::Result<String> {
    use std::path::Path;

    eprintln!("{} {}", "reading the prisma schema from".bold(), path.yellow());

    let path = Path::new(path);
    let mut file = File::open(path)?;

    let mut out = String::new();
    file.read_to_string(&mut out)?;

    Ok(out)
}

fn read_datamodel_from_stdin() -> std::io::Result<String> {
    use std::io::Read;

    eprintln!("{} {}", "reading the prisma schema from".bold(), "stdin".yellow());

    let mut stdin = std::io::stdin();

    let mut out = String::new();
    stdin.read_to_string(&mut out)?;

    Ok(out)
}

fn minimal_schema_from_url(url: &str) -> anyhow::Result<String> {
    let provider = match url.split(':').next() {
        Some("file") | Some("sqlite") => "sqlite",
        Some(s) if s.starts_with("postgres") => "postgresql",
        Some("mysql") => "mysql",
        _ => anyhow::bail!("Could not extract a provider from the URL"),
    };

    let schema = format!(
        r#"
            datasource db {{
                provider = "{}"
                url = "{}"
            }}
        "#,
        provider, url
    );

    Ok(schema)
}

async fn generate_dmmf(cmd: &DmmfCommand) -> anyhow::Result<()> {
    let schema_path: String = {
        if let Some(url) = cmd.url.as_ref() {
            let skeleton = minimal_schema_from_url(url)?;
            //todo make this configurable
            let introspected = introspection_core::RpcImpl::introspect_internal(skeleton, false)
                .await
                .map_err(|err| anyhow::anyhow!("{:?}", err.data))?;

            eprintln!("{}", "Schema was successfully introspected from database URL".green());

            let path = "/tmp/prisma-test-cli-introspected.prisma";
            std::fs::write(path, introspected.datamodel)?;
            path.to_owned()
        } else if let Some(file_path) = cmd.file_path.as_ref() {
            file_path.clone()
        } else {
            eprintln!(
                "{} {} {} {}",
                "Please provide one of".yellow(),
                "--url".bold(),
                "or".yellow(),
                "--file-path".bold()
            );
            std::process::exit(1)
        }
    };

    eprintln!(
        "{} {}",
        "Using the query engine binary at".yellow(),
        cmd.query_engine_binary_path.bold()
    );

    let cmd = std::process::Command::new(&cmd.query_engine_binary_path)
        .arg("cli")
        .arg("dmmf")
        .env("PRISMA_DML_PATH", schema_path)
        .spawn()?;

    cmd.wait_with_output()?;

    Ok(())
}

fn init_logger() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;

    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .finish()
        .with(ErrorLayer::default());

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| eprintln!("Error initializing the global logger: {}", err))
        .ok();
}

async fn push_schema(cmd: &PushSchema) -> anyhow::Result<()> {
    let schema = read_datamodel_from_file(&cmd.schema_path).context("Error reading the schema from file")?;
    let api = migration_core::migration_api(&schema).await?;

    let response = api
        .push_schema(&PushSchemaInput {
            schema,
            force: cmd.force,
        })
        .await?;

    if response.warnings.len() > 0 {
        eprintln!("⚠️  {}", "Warnings".bright_yellow().bold());

        for warning in &response.warnings {
            eprintln!("- {}", warning.bright_yellow())
        }
    }

    if response.unexecutable.len() > 0 {
        eprintln!("☢️  {}", "Unexecutable steps".bright_red().bold());

        for unexecutable in &response.unexecutable {
            eprintln!("- {}", unexecutable.bright_red())
        }
    }

    if response.executed_steps > 0 {
        eprintln!(
            "{}  {}",
            "✔️".bold(),
            format!("Schema pushed to database. ({} steps)", response.executed_steps).green()
        );
    } else {
        if response.had_no_changes_to_push() {
            eprintln!(
                "{}  {}",
                "✔️".bold(),
                "No changes to push. Prisma schema and database are in sync.".green()
            );
        } else {
            eprintln!(
                "{}  {}",
                "❌".bold(),
                "The schema was not pushed. Pass the --force flag to ignore warnings."
            );
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn generate_migration(cmd: &MigrateSave) -> anyhow::Result<()> {
    let target_schema = read_datamodel_from_file(&cmd.schema_path)?;
    let migrations = read_migrations_from_folder(&cmd.migrations_folder_path)?;

    let api = migration_core::migration_api(&target_schema).await?;

    let input = GenerateImperativeMigrationInput {
        target_schema,
        migrations,
        migration_name: cmd.migration_name.clone(),
    };

    let response = api.generate_imperative_migration(&input).await?;

    if response.warnings.len() > 0 {
        eprintln!("⚠️  {}", "Warnings".bright_yellow().bold());

        for warning in &response.warnings {
            eprintln!("- {}", warning.bright_yellow())
        }
    }

    if response.unexecutable.len() > 0 {
        eprintln!("☢️  {}", "Unexecutable steps".bright_red().bold());

        for unexecutable in &response.unexecutable {
            eprintln!("- {}", unexecutable.bright_red())
        }
    }

    if response.migration.is_empty() {
        eprintln!(
            "{}  {}",
            "✔️".bold(),
            "The migrations are up-to-date, no new migration was generated.".green()
        );

        return Ok(());
    }

    let migration_file_path =
        std::path::Path::new(&cmd.migrations_folder_path).join(&format!("{}.json", response.migration.name));
    let file = std::fs::File::create(&migration_file_path)?;
    serde_json::to_writer_pretty(file, &response.migration)?;

    eprintln!(
        "{}  {}",
        "✔️".bold(),
        format!("Migration written to {}.", migration_file_path.to_string_lossy()).green()
    );

    Ok(())
}

fn read_migrations_from_folder(path: &str) -> Result<Vec<ImperativeMigration>, anyhow::Error> {
    let mut migrations = Vec::new();

    for entry in std::fs::read_dir(path).context("error reading from migrations directory")? {
        let entry = entry?;

        if !entry.file_type()?.is_file() {
            continue;
        }

        let file = File::open(entry.path()).context("error opening a migration file")?;
        migrations.push(serde_json::from_reader(file).context("error deserializing a migration")?);
    }

    // Ensure migrations are ordered. This is not guaranteed by read_dir().
    migrations.sort_by(|a: &ImperativeMigration, b| a.name.cmp(&b.name));

    Ok(migrations)
}
