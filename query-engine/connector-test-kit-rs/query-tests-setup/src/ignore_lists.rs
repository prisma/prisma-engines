use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::OnceLock,
};

static IGNORED_TESTS: OnceLock<HashSet<String>> = OnceLock::new();
static SHOULD_FAIL_TESTS: OnceLock<HashSet<String>> = OnceLock::new();

pub fn is_ignored(test_name: &str) -> bool {
    is_in_list(test_name, "IGNORED_TESTS", &IGNORED_TESTS)
}

pub fn is_expected_to_fail(test_name: &str) -> bool {
    is_in_list(test_name, "SHOULD_FAIL_TESTS", &SHOULD_FAIL_TESTS)
}

fn is_in_list(test_name: &str, env_var: &'static str, cache: &OnceLock<HashSet<String>>) -> bool {
    let list_file = match std::env::var(env_var) {
        Ok(file) => file,
        Err(_) => return false,
    };

    let tests = cache.get_or_init(|| {
        let workspace_root = std::env::var("WORKSPACE_ROOT").expect("WORKSPACE_ROOT env must be set");

        let path = PathBuf::from(workspace_root).join(list_file);
        let file = File::open(path).expect("could not open file");
        let reader = BufReader::new(file);

        reader
            .lines()
            .map(|line| line.expect("could not read line"))
            .map(|line| {
                let trimmed = line.trim();
                if line != trimmed {
                    trimmed.to_owned()
                } else {
                    line
                }
            })
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect()
    });

    tests.contains(test_name)
}
