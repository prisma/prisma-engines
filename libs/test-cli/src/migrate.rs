use colored::Colorize;
use enumflags2::BitFlags;
use migration_core::commands::{
    ApplyMigrationsInput, CreateMigrationInput, DevAction, DevDiagnosticInput, EvaluateDataLossInput,
};
use rustyline::Editor;

pub(crate) async fn handle(migrate: &crate::Migrate) -> anyhow::Result<()> {
    let schema_path = migrate.schema_path.as_deref().unwrap_or("./prisma/schema.prisma");
    let schema = crate::read_datamodel_from_file(schema_path)?;
    let api = migration_core::migration_api(&schema, BitFlags::all()).await?;
    let migrations_directory_path = "./prisma/migrations".to_owned();
    let mut rl = Editor::<()>::new();

    match &migrate.command {
        crate::MigrateCommand::Dev(dev) => {
            let dev_action = api
                .dev_diagnostic(&DevDiagnosticInput {
                    migrations_directory_path: migrations_directory_path.clone(),
                })
                .await?;

            match dev_action.action {
                DevAction::Reset { reason } => {
                    eprintln!("Maybe we should reset!\n\n{}\n\n", reason);

                    let line = rl.readline("Reset? (y/n) ")?;

                    match line.to_lowercase().trim() {
                        "y" => {
                            api.reset(&()).await?;
                        }
                        "n" => {
                            eprintln!("Ok then. Not doing anything.");

                            return Ok(());
                        }
                        _ => anyhow::bail!("Choice was neither `y` nor `n`."),
                    }
                }
                DevAction::CreateMigration => {}
            }

            let apply_response = api
                .apply_migrations(&ApplyMigrationsInput {
                    migrations_directory_path: migrations_directory_path.clone(),
                })
                .await?;

            if !apply_response.applied_migration_names.is_empty() {
                eprintln!(
                    "Applied migrations: {}",
                    apply_response.applied_migration_names.join(", ")
                );
            }

            // Evaluate data loss
            let response = api
                .evaluate_data_loss(&EvaluateDataLossInput {
                    prisma_schema: schema.clone(),
                    migrations_directory_path: migrations_directory_path.clone(),
                })
                .await?;

            if !response.warnings.is_empty() {
                eprintln!("⚠️  {}", "Warnings".bright_yellow().bold());

                for warning in &response.warnings {
                    eprintln!("- {}", warning.message.bright_yellow())
                }
            }

            if !response.unexecutable_steps.is_empty() {
                eprintln!("☢️  {}", "Unexecutable steps".bright_red().bold());

                for unexecutable in &response.unexecutable_steps {
                    eprintln!("- {}", unexecutable.message.bright_red())
                }

                return Ok(());
            }

            if !response.warnings.is_empty() && !dev.create_only {
                match rl.readline("Proceed anyway? (y/n) ")?.trim().to_lowercase().as_str() {
                    "y" => (),
                    _ => return Ok(()),
                }
            }

            let response = api
                .create_migration(&CreateMigrationInput {
                    migrations_directory_path: migrations_directory_path.clone(),
                    prisma_schema: schema,
                    migration_name: dev.name.clone(),
                    draft: dev.create_only,
                })
                .await?;

            match response.generated_migration_name {
                Some(name) => {
                    eprintln!("Generated `{}`.", name);
                }
                None => {
                    eprintln!("No change to commit to a migration.");
                    return Ok(());
                }
            }

            if !dev.create_only {
                api.apply_migrations(&ApplyMigrationsInput {
                    migrations_directory_path,
                })
                .await?;

                eprintln!("Migration applied.")
            }

            Ok(())
        }
        crate::MigrateCommand::Deploy => {
            api.apply_migrations(&ApplyMigrationsInput {
                migrations_directory_path,
            })
            .await?;

            Ok(())
        }
        crate::MigrateCommand::Reset => {
            match rl.readline("Reset? (y/n) ")?.trim().to_lowercase().as_str() {
                "y" => (),
                _ => {
                    eprintln!("Reset cancelled.");
                    return Ok(());
                }
            }

            api.reset(&()).await?;

            eprintln!("Reset done.");

            Ok(())
        }
    }
}
