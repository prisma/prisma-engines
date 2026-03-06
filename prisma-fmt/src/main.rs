mod actions;
mod format;
mod native;
mod preview;
mod schema_file_input;

use std::{
    io::{self, Read},
    path::PathBuf,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub struct FormatOpts {
    /// Specifies the input file to use. If none is given, the input is read
    /// from STDIN
    #[structopt(short = "i", long)]
    input: Option<PathBuf>,
    /// Specifies the output file to use. If none is given, the output is
    /// written to STDOUT
    #[structopt(short = "o", long)]
    output: Option<PathBuf>,
    /// Specifies which tab width to use when formatting
    #[structopt(short = "s", long, default_value = "2")]
    tabwidth: usize,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(version = env!("GIT_HASH"))]
/// Prisma Datamodel v2 formatter
pub enum FmtOpts {
    /// Specifies linter mode
    // Lint,
    /// Specifies format mode
    Format(FormatOpts),
    /// Specifies Native Types mode
    NativeTypes,
    /// List of available referential actions
    ReferentialActions,
    /// Specifies preview features mode
    PreviewFeatures,
    /// Artificially panic (for testing the CLI)
    DebugPanic,
    /// Generate DMMF JSON from schema, streaming to stdout.
    /// Reads JSON params from stdin, streams DMMF to stdout via serde_json::to_writer().
    /// This has no memory ceiling — unlike WASM, the binary can stream arbitrarily large DMMF.
    /// See: https://github.com/prisma/prisma/issues/29111
    GetDmmf,
}

fn main() {
    match FmtOpts::from_args() {
        FmtOpts::DebugPanic => panic!("This is the debugPanic artificial panic"),
        // FmtOpts::Lint => plug(lint::run),
        FmtOpts::Format(opts) => format::run(opts),
        FmtOpts::NativeTypes => plug(native::run),
        FmtOpts::ReferentialActions => plug(actions::run),
        FmtOpts::PreviewFeatures => plug(|_s| preview::run()),
        FmtOpts::GetDmmf => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .expect("Unable to read from stdin.");
            let stdout = io::stdout();
            let writer = io::BufWriter::new(stdout.lock());
            if let Err(e) = prisma_fmt::get_dmmf_to_writer(&input, writer) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
}

fn plug<F: Fn(&str) -> String>(f: F) {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    print!("{}", f(&datamodel_string))
}
