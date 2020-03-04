mod format;
mod lint;

use std::path::PathBuf;

use serde;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub struct LintOpts {
    /// If set, silences all `environment variable not found` errors
    #[structopt(long)]
    no_env_errors: bool,
}

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
    /// Specifies wich tab width to use when formatting
    #[structopt(short = "s", long, default_value = "2")]
    tabwidth: usize,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(version = env!("GIT_HASH"))]
/// Prisma Datamodel v2 formatter
pub enum FmtOpts {
    /// Specifies linter mode
    Lint(LintOpts),
    /// Specifies format mode
    Format(FormatOpts),
}

#[derive(serde::Serialize)]
pub struct MiniError {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

fn main() {
    match FmtOpts::from_args() {
        FmtOpts::Lint(opts) => lint::run(opts),
        FmtOpts::Format(opts) => format::run(opts),
    }
}
