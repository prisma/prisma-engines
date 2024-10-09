use enumflags2::BitFlags;
use once_cell::sync::Lazy;
use quaint::{prelude::Queryable, single::Quaint};

use crate::{runtime::run_with_thread_local_runtime as tok, Tags};

pub fn sqlite_test_url(db_name: &str) -> String {
    std::env::var("SQLITE_TEST_URL").unwrap_or_else(|_| format!("file:{}", sqlite_test_file(db_name)))
}

fn sqlite_test_file(db_name: &str) -> String {
    static WORKSPACE_ROOT: Lazy<std::path::PathBuf> = Lazy::new(|| {
        std::env::var("WORKSPACE_ROOT")
            .map(|root| std::path::Path::new(&root).join("db"))
            .unwrap_or_else(|_| {
                let dir = std::env::temp_dir().join("prisma_tests_workspace_root");
                let path = dir.to_string_lossy().into_owned();

                std::fs::create_dir_all(&path).expect("failed to create WORKSPACE_ROOT directory");

                path.into()
            })
    });

    let file_path = WORKSPACE_ROOT.join(db_name);

    // Truncate the file.
    std::fs::File::create(&file_path).expect("Failed to create or truncate SQLite database.");

    file_path.to_string_lossy().into_owned()
}

pub(crate) fn get_sqlite_tags() -> Result<BitFlags<Tags>, String> {
    let fut = async {
        let mut tags: BitFlags<Tags> = Tags::Sqlite.into();
        // The SpatiaLite extension is loaded by quaint, assuming the SPATIALITE_PATH env variable is set
        // If the extension can be loaded in a dummy database, it means it will also be available for the tests
        let quaint = Quaint::new_in_memory().map_err(|err| err.to_string())?;
        if let Ok(_has_spatialite) = quaint.query_raw("SELECT spatialite_version();", &[]).await {
            tags |= Tags::Spatialite;
        }
        Ok(tags)
    };
    tok(fut)
}
