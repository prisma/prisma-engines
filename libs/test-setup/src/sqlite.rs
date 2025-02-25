use std::sync::LazyLock;

pub fn sqlite_test_url(db_name: &str) -> String {
    std::env::var("SQLITE_TEST_URL").unwrap_or_else(|_| format!("file:{}", sqlite_test_file(db_name)))
}

fn sqlite_test_file(db_name: &str) -> String {
    static WORKSPACE_ROOT: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
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
