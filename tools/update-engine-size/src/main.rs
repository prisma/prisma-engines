use std::{
    fs::{self, OpenOptions},
    path::Path,
};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Args {
    /// Path to the JSON database
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

#[derive(Serialize, Deserialize)]
struct DbEntry {
    date_time: String,
    branch: String,
    commit: String,
    file: String,
    size_bytes: u64,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
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
            date_time: date_time.clone(),
            branch: args.branch.clone(),
            commit: args.commit.clone(),
            file: path.file_name().unwrap().to_string_lossy().into_owned(),
            size_bytes: fs::metadata(path)?.len(),
        };

        writer.serialize(&entry)?;
    }

    Ok(())
}
