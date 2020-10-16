use anyhow::Context;
use colored::Colorize;
use migration_core::commands::SchemaPushInput;
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
    SchemaPush(SchemaPush),
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

#[derive(StructOpt)]
struct SchemaPush {
    schema_path: String,
    #[structopt(long)]
    force: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logger();

    match Command::from_args() {
        Command::Dmmf(cmd) => generate_dmmf(&cmd).await?,
        Command::SchemaPush(cmd) => schema_push(&cmd).await?,
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

            migration_core::migration_api(&datamodel_string)
                .await?
                .reset(&())
                .await?;

            let api = migration_core::migration_api(&datamodel_string).await?;
            let migration_id = "test-cli-migration".to_owned();

            let infer_input = migration_core::InferMigrationStepsInput {
                assume_applied_migrations: Some(Vec::new()),
                assume_to_be_applied: Some(Vec::new()),
                datamodel: datamodel_string.clone(),
                migration_id: migration_id.clone(),
            };

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
    eprintln!("{} {}", "reading the prisma schema from".bold(), "stdin".yellow());

    let mut stdin = std::io::stdin();

    let mut out = String::new();
    stdin.read_to_string(&mut out)?;

    Ok(out)
}

fn minimal_schema_from_url(url: &str) -> anyhow::Result<String> {
    let provider = match url.split("://").next() {
        Some("file") | Some("sqlite") => "sqlite",
        Some(s) if s.starts_with("postgres") => "postgresql",
        Some("mysql") => "mysql",
        Some("sqlserver") | Some("jdbc:sqlserver") => "sqlserver",
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

async fn schema_push(cmd: &SchemaPush) -> anyhow::Result<()> {
    let schema = read_datamodel_from_file(&cmd.schema_path).context("Error reading the schema from file")?;
    let api = migration_core::migration_api(&schema).await?;

    let response = api
        .schema_push(&SchemaPushInput {
            schema,
            force: cmd.force,
            assume_empty: false,
        })
        .await?;

    if !response.warnings.is_empty() {
        eprintln!("⚠️  {}", "Warnings".bright_yellow().bold());

        for warning in &response.warnings {
            eprintln!("- {}", warning.bright_yellow())
        }
    }

    if !response.unexecutable.is_empty() {
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
    } else if response.had_no_changes_to_push() {
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
