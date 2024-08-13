use std::path::Path;

pub fn load_schema_files(dir: impl AsRef<Path>) -> Vec<(String, String)> {
    let schema_files = {
        std::fs::read_dir(dir.as_ref())
            .unwrap()
            .map(Result::unwrap)
            .filter_map(|entry| {
                let ft = entry.file_type().ok()?;
                if ft.is_dir() {
                    return None;
                }
                let path = entry.path();
                let name = path.file_name()?.to_str()?;
                let ext = path.extension()?;
                if ext != "prisma" {
                    return None;
                }

                Some((
                    format!("file:///path/to/{name}"),
                    std::fs::read_to_string(&path).unwrap(),
                ))
            })
            .collect::<Vec<_>>()
    };
    assert!(!schema_files.is_empty());

    schema_files
}
