mod panic_with_diff;

use std::{fs, io::Write as _, path};

const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reformatter");

#[inline(never)] // we want to compile fast
fn run_reformat_test(test_file_path: &str) {
    let file_path = path::Path::new(TESTS_ROOT).join(test_file_path);
    let text = fs::read_to_string(file_path).unwrap();
    let reformatted_text: String = reformat(&text);

    let snapshot_file_name = path::Path::new(TESTS_ROOT).join(format!(
        "{}.reformatted.prisma",
        test_file_path.trim_end_matches(".prisma")
    ));
    let expected_text: String = fs::read_to_string(&snapshot_file_name).unwrap_or_default();

    if reformatted_text == expected_text {
        return; // test passed
    }

    if std::env::var("UPDATE_EXPECT").is_ok() {
        let mut file = fs::File::create(&snapshot_file_name).unwrap(); // truncate
        file.write_all(reformatted_text.as_bytes()).unwrap();
    } else {
        panic_with_diff::panic_with_diff(&expected_text, &reformatted_text, None);
    }

    if reformat(&reformatted_text) != reformatted_text {
        println!("=== reformatted ===\n{reformatted_text}");
        println!("=== reformatted again ===\n{}", reformat(&reformatted_text));
        panic!("Reformatting this schema is not idempotent.");
    }
}

include!(concat!(env!("OUT_DIR"), "/reformat_tests.rs"));

fn reformat(s: &str) -> String {
    psl::reformat(s, 2).unwrap()
}

mod reformat_multi_file {
    use std::{collections::HashMap, fs, io::Write, path};

    use psl::{reformat_multiple, SourceFile};

    use crate::panic_with_diff;

    const MULTIFILE_TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reformatter_multi_file");

    #[inline(never)]
    fn run_reformat_multi_file_test(test_dir_name: &str) {
        let dir_path = path::Path::new(MULTIFILE_TESTS_ROOT).join(test_dir_name);
        let snapshot_dir_path = path::Path::new(MULTIFILE_TESTS_ROOT).join(format!("{test_dir_name}.reformatted"));

        fs::create_dir_all(&snapshot_dir_path).unwrap();
        let schemas: Vec<_> = read_schemas_from_dir(dir_path).collect();

        let result = reformat_multiple(schemas, 2);

        let should_update = std::env::var("UPDATE_EXPECT").is_ok();
        let mut snapshot_schemas: HashMap<_, _> = read_schemas_from_dir(&snapshot_dir_path).collect();
        for (path, content) in result {
            let content = content.as_str();
            let snapshot_content = snapshot_schemas.remove(&path).unwrap_or_default();
            let snapshot_content = snapshot_content.as_str();
            if content == snapshot_content {
                continue;
            }

            if should_update {
                let snapshot_file_path = path::Path::new(&snapshot_dir_path).join(path);
                let mut file = fs::File::create(&snapshot_file_path).unwrap();
                file.write_all(content.as_bytes()).unwrap()
            } else {
                panic_with_diff::panic_with_diff(snapshot_content, content, Some(&path));
            }
        }

        // cleanup removed files
        for missing_file in snapshot_schemas.keys() {
            if should_update {
                fs::remove_file(path::Path::new(&snapshot_dir_path).join(missing_file)).unwrap()
            } else {
                panic!("{missing_file} is present in the snapshot directory, but missing from formatting results")
            }
        }
    }

    fn read_schemas_from_dir(root_dir_path: impl AsRef<path::Path>) -> impl Iterator<Item = (String, SourceFile)> {
        let root_dir_path = root_dir_path.as_ref().to_owned();
        fs::read_dir(&root_dir_path)
            .unwrap()
            .map(Result::unwrap)
            .filter_map(move |entry| {
                let file_name = entry.file_name();
                let file_name = file_name.to_str().unwrap();
                if !file_name.ends_with(".prisma") {
                    None
                } else {
                    let full_path = root_dir_path.clone().join(file_name);
                    let content = fs::read_to_string(full_path).unwrap();
                    Some((file_name.to_owned(), content.into()))
                }
            })
    }

    include!(concat!(env!("OUT_DIR"), "/reformat_multi_file_tests.rs"));
}
