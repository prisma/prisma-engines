use anyhow::Context;
use colored::Colorize;
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Command::from_args() {
        Command::Introspect { url, file_path } => {
            if url.as_ref().xor(file_path.as_ref()).is_none() {
                anyhow::bail!(
                    "{}",
                    "Exactly one of --url or --file-path must be provided"
                        .bold()
                        .red()
                );
            }

            let schema = if let Some(file_path) = file_path {
                read_datamodel_from_file(&file_path)?
            } else if let Some(url) = url {
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

                schema
            } else {
                unreachable!()
            };

            let introspected = introspection_core::RpcImpl::introspect_internal(schema)
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
                (Some(path), false) => {
                    read_datamodel_from_file(&path).context("error reading the schemafile")?
                }
                (None, true) => read_datamodel_from_stdin()?,
                (Some(_), true) => anyhow::bail!(
                    "{}",
                    "please pass either --stdin or --file-path, not both"
                        .bold()
                        .red()
                ),
                (None, false) => anyhow::bail!(
                    "{}",
                    "either --stdin or --file-path is required".bold().red()
                ),
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
            let warnings: Vec<_> = result
                .warnings
                .into_iter()
                .map(|warning| warning.description)
                .collect();

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
    use std::{fs::File, io::Read, path::Path};

    eprintln!(
        "{} {}",
        "reading the prisma schema from".bold(),
        path.yellow()
    );

    let path = Path::new(path);
    let mut file = File::open(path)?;

    let mut out = String::new();
    file.read_to_string(&mut out)?;

    Ok(out)
}

fn read_datamodel_from_stdin() -> std::io::Result<String> {
    use std::io::Read;

    eprintln!(
        "{} {}",
        "reading the prisma schema from".bold(),
        "stdin".yellow()
    );

    let mut stdin = std::io::stdin();

    let mut out = String::new();
    stdin.read_to_string(&mut out)?;

    Ok(out)
}
