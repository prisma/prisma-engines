use std::{
    error::Error,
    fs::{self, OpenOptions},
    path::Path,
};

use clap::Parser;
use serde::Serialize;

/// This tool updates the size of the engine binaries in the CSV file
/// on CI for tracking their changes over time.
#[derive(Parser)]
struct Args {
    /// Path to the CSV database
    #[arg(long)]
    db: String,

    /// Current git branch
    #[arg(long)]
    branch: String,

    /// Current git commit
    #[arg(long)]
    commit: String,

    /// List of engine files
    files: Vec<String>,
}

#[derive(Serialize)]
struct DbEntry<'a> {
    date_time: &'a str,
    branch: &'a str,
    commit: &'a str,
    file: &'a str,
    size_bytes: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.files.is_empty() {
        panic!("Please provide at least one engine file");
    }

    let date_time = chrono::Utc::now().to_rfc3339();
    let write_headers = !Path::new(&args.db).exists();

    let csv_file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&args.db)?;

    let mut writer = csv::WriterBuilder::new()
        .has_headers(write_headers)
        .from_writer(csv_file);

    for path in args.files {
        let path = Path::new(&path);

        let entry = DbEntry {
            date_time: &date_time,
            branch: &args.branch,
            commit: &args.commit,
            file: &path.file_name().unwrap().to_string_lossy(),
            size_bytes: fs::metadata(path)?.len(),
        };

        writer.serialize(&entry)?;
    }

    Ok(())
}
