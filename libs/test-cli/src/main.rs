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
}

fn main() -> anyhow::Result<()> {
    match Command::from_args() {
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

            let fut = async move {
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
                let warnings = result.warnings.into_iter().map(|warning| warning.description).collect();

                Ok::<Vec<String>, anyhow::Error>(warnings)
            };

            let mut rt = tokio::runtime::Runtime::new()?;

            let warnings = rt.block_on(fut)?;

            if warnings.is_empty() {
                println!("{}", "✔️  migrated without warning".bold().green());
            } else {
                for warning in warnings {
                    println!("{} - {}", "⚠️ MIGRATION WARNING ⚠️ ".bold().red(), warning)
                }

                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn read_datamodel_from_file(path: &str) -> std::io::Result<String> {
    use std::{fs::File, io::Read, path::Path};

    println!("{} {}", "reading the prisma schema from".bold(), path.yellow());

    let path = Path::new(path);
    let mut file = File::open(path)?;

    let mut out = String::new();
    file.read_to_string(&mut out)?;

    Ok(out)
}

fn read_datamodel_from_stdin() -> std::io::Result<String> {
    use std::io::Read;

    println!("{} {}", "reading the prisma schema from".bold(), "stdin".yellow());

    let mut stdin = std::io::stdin();

    let mut out = String::new();
    stdin.read_to_string(&mut out)?;

    Ok(out)
}
