#![feature(trait_upcasting)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use psl::builtin_connectors::{BUILTIN_CONNECTORS, MYSQL, POSTGRES, SQLITE};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    // Given a textual `.prisma` file, read it, validate it, print its `ValidatedSchema`, and write it to a `.bin` file.
    Serialize {
        /// lists test values
        #[arg(short, long)]
        file: PathBuf,

        /// lists test values
        #[arg(short, long)]
        output: PathBuf,
    },
    // Given a binary `.bin` file, read it as a `ValidatedSchema`, and print it.
    Deserialize {
        /// lists test values
        #[arg(short, long)]
        file: PathBuf,
    },
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match cli.command {
        Some(Commands::Serialize { file, output }) => {
            println!("Serializing {:?}...", &file.as_os_str());

            let schema_as_text = std::fs::read_to_string(file)?;

            let schema_as_bytes = psl::serialize_to_bytes(schema_as_text.into(), BUILTIN_CONNECTORS)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            println!("Writing to {:?}", output);
            std::fs::write(output, schema_as_bytes)?;

            Ok(())
        }
        Some(Commands::Deserialize { file }) => {
            println!("Deserializing {:?}...", &file.as_os_str());

            let schema_as_binary = std::fs::read(file)?;

            let connector_registry: psl::ValidatedConnectorRegistry<'_> = &[POSTGRES, MYSQL, SQLITE];

            let schema_qe = psl::deserialize_from_bytes(schema_as_binary.as_slice(), &connector_registry)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            println!("connector.provider_name(): {}", &schema_qe.connector.provider_name());

            Ok(())
        }
        None => {
            print!("No subcommand provided. Exiting.");
            Ok(())
        }
    }
}
