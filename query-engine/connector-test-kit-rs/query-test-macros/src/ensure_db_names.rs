use std::collections::HashSet;
use std::sync::Mutex;

// Ensures unique test database names.
// Reusing the same name more than once causes random test failures, retries and overall flakiness.
pub struct UniqueTestDatabaseNames {
    names: Mutex<HashSet<String>>,
}

impl UniqueTestDatabaseNames {
    pub fn new() -> Self {
        Self {
            names: Mutex::new(HashSet::new()),
        }
    }

    pub fn ensure_unique(
        self: &UniqueTestDatabaseNames,
        test_database_name: &String,
        suite_name: &String,
        test_name: &String,
    ) {
        match self.names.lock() {
            Ok(mut names) => {
                if names.contains(test_database_name) {
                    panic!(
                        "Test database (or schema) names must be unique.\n\
                         It is concatenation of the test suite and the test case names.\n\
                         To resolve this error rename them until they are unique.\n\
                         - Test suite: `{}`\n\
                         - Test case: `{}`\n\
                         - Database (or schema): `{}`\n",
                        suite_name, test_name, test_database_name
                    );
                }
                names.insert(test_database_name.to_string());
            }

            Err(_) => {
                // Ignore poisoned RwLock, when another thread has already panicked.
                // This prevents spamming the error output.
            }
        }

        if test_database_name.len() > 64 {
            panic!(
                "Test database (or schema) names must be at most 64 characters \
                 for PostgreSQL compatibility.\n\
                 To resolve this error shorten the name of the test suite and/or test case.\n\
                 - Test suite: `{}`\n\
                 - Test case: `{}`\n\
                 - Database (or schema): `{}`\n",
                suite_name, test_name, test_database_name
            );
        }
    }
}
