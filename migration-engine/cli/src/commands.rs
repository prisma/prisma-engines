pub(crate) mod error;

#[cfg(test)]
mod tests;

use enumflags2::BitFlags;
use error::CliError;
use migration_core::{api::RpcApi, migration_api, qe_setup::QueryEngineFlags, CoreError};
use pico_args::Arguments;
use user_facing_errors::{
    common::{InvalidDatabaseString, SchemaParserError},
    KnownError, UserFacingError,
};

pub(crate) async fn run(subcommand: &str, mut args: pico_args::Arguments) -> Result<(), CliError> {
    match subcommand {
        "create-database" => {
            let datasource: String = args.value_from_str("datasource").unwrap();

            create_database(&datasource).await
        }
        "can-connect-to-database" => {
            let datasource: String = args.value_from_str("datasource").unwrap();

            connect_to_database(&datasource).await
        }
        "drop-database" => {
            let datasource: String = args.value_from_str("datasource").unwrap();

            drop_database(&datasource).await
        }
        "qe-setup" => {
            let datasource: String = args.value_from_str("datasource").unwrap();

            qe_setup(&datasource, args).await
        }
        "start" => {
            let datamodel_location = match args.value_from_str::<_, String>("--datamodel") {
                Ok(loc) => loc,
                Err(_) => {
                    eprintln!("The --datamodel argument is required.\n");
                    crate::print_help_text()
                }
            };
            start_engine(&datamodel_location).await
        }
        other => {
            eprintln!("Unknown subcommand `{}`", other);
            crate::print_help_text();
        }
    }
}

async fn connect_to_database(database_str: &str) -> Result<(), CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    migration_api(&datamodel).await?;

    Ok(println!("Connection successful"))
}

async fn create_database(database_str: &str) -> Result<(), CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    let db_name = migration_core::create_database(&datamodel).await?;

    Ok(println!("Database '{}' was successfully created.", db_name))
}

async fn drop_database(database_str: &str) -> Result<(), CliError> {
    let datamodel = datasource_from_database_str(database_str)?;
    migration_core::drop_database(&datamodel).await?;

    Ok(println!("The database was successfully dropped."))
}

async fn qe_setup(prisma_schema: &str, mut args: Arguments) -> Result<(), CliError> {
    fn parse_setup_flags(s: &str) -> Result<BitFlags<QueryEngineFlags>, CliError> {
        let mut flags = BitFlags::empty();

        for flag in s.split(',') {
            match flag {
                "database_creation_not_allowed" => flags.insert(QueryEngineFlags::DatabaseCreationNotAllowed),
                "" => (),
                flag => return Err(CoreError::from_msg(format!("Unknown flag: {}", flag)).into()),
            }
        }

        Ok(flags)
    }

    let flags = args
        .opt_value_from_fn("qe-test-setup-flags", parse_setup_flags)
        .unwrap()
        .unwrap_or_else(BitFlags::empty);

    Ok(migration_core::qe_setup::run(&prisma_schema, flags).await?)
}

async fn start_engine(datamodel_location: &str) -> ! {
    use std::io::Read as _;

    tracing::info!(git_hash = env!("GIT_HASH"), "Starting migration engine RPC server",);
    let mut file = std::fs::File::open(datamodel_location).expect("error opening datamodel file");

    let mut datamodel = String::new();
    file.read_to_string(&mut datamodel).unwrap();

    match RpcApi::new(&datamodel).await {
        // Block the thread and handle IO in async until EOF.
        Ok(api) => json_rpc_stdio::run(api.io_handler()).await.unwrap(),
        Err(err) => {
            let user_facing_error = err.to_user_facing();
            let exit_code =
                if user_facing_error.as_known().map(|err| err.error_code) == Some(SchemaParserError::ERROR_CODE) {
                    1
                } else {
                    250
                };

            serde_json::to_writer(std::io::stdout().lock(), &user_facing_error).expect("failed to write to stdout");
            std::process::exit(exit_code)
        }
    }

    std::process::exit(0);
}

fn datasource_from_database_str(database_str: &str) -> Result<String, CliError> {
    let provider = match database_str.split(':').next() {
        Some("postgres") => "postgresql",
        Some("file") => "sqlite",
        Some(other) => other,
        None => {
            return Err(CliError::Known {
                error: KnownError::new(InvalidDatabaseString { details: String::new() }),
                exit_code: 1,
            })
        }
    };

    Ok(format!(
        r#"
            datasource db {{
                provider = "{provider}"
                url = "{url}"
            }}
        "#,
        provider = provider,
        url = database_str,
    ))
}
