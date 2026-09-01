//! Deciding whether two connection strings point at the same database.

use psl::datamodel_connector::Flavour;

/// Returns `true` when both connection strings denote the same database, i.e. when writing to one
/// of them is observable through the other.
///
/// The comparison is flavour-aware and normalizing: it parses both connection strings the way the
/// respective connector does and compares the host (case-insensitively), the port (with the
/// flavour's default applied) and the database name. Everything else — credentials, connection
/// pool settings, TLS parameters and the PostgreSQL `schema` selector — is ignored, so two
/// connection strings that differ only in those still denote the same database.
///
/// When either connection string cannot be parsed, the comparison degrades to exact string
/// equality: a caller that refuses to proceed on `true` must not refuse a working configuration
/// because of a parsing quirk, and a connection string that is genuinely broken fails when the
/// connection is opened.
pub fn urls_denote_same_database(flavour: Flavour, first: &str, second: &str) -> bool {
    compare_urls(flavour, first, second).unwrap_or(first == second)
}

/// `None` means "cannot tell", which leaves the decision to the caller of the comparison.
// With no flavour enabled there is no arm left to compare anything in.
#[cfg_attr(
    not(any(
        feature = "postgresql",
        feature = "cockroachdb",
        feature = "mysql",
        feature = "mssql",
        feature = "sqlite"
    )),
    allow(unused_variables)
)]
fn compare_urls(flavour: Flavour, first: &str, second: &str) -> Option<bool> {
    match flavour {
        #[cfg(any(feature = "postgresql", feature = "cockroachdb"))]
        Flavour::Postgres | Flavour::Cockroach => compare_postgres_urls(first, second),

        #[cfg(feature = "mysql")]
        Flavour::Mysql => compare_mysql_urls(first, second),

        #[cfg(feature = "mssql")]
        Flavour::Sqlserver => compare_mssql_urls(first, second),

        #[cfg(feature = "sqlite")]
        Flavour::Sqlite => compare_sqlite_urls(first, second),

        _ => None,
    }
}

#[cfg(any(feature = "postgresql", feature = "cockroachdb"))]
fn compare_postgres_urls(first: &str, second: &str) -> Option<bool> {
    let first = parse_postgres_url(first)?;
    let second = parse_postgres_url(second)?;

    Some(hosts_match(first.host(), second.host()) && first.port() == second.port() && first.dbname() == second.dbname())
}

#[cfg(any(feature = "postgresql", feature = "cockroachdb"))]
fn parse_postgres_url(url: &str) -> Option<quaint::connector::PostgresNativeUrl> {
    const PRISMA_POSTGRES_SCHEME: &str = "prisma+postgres";

    let url: url::Url = url.parse().ok()?;

    // Prisma Postgres connection strings identify the database through the `api_key` parameter
    // rather than through the host and the path, so two of them that agree on host, port and
    // database name can still point at different databases.
    if url.scheme() == PRISMA_POSTGRES_SCHEME {
        return None;
    }

    quaint::connector::PostgresNativeUrl::new(url).ok()
}

#[cfg(feature = "mysql")]
fn compare_mysql_urls(first: &str, second: &str) -> Option<bool> {
    let first = parse_mysql_url(first)?;
    let second = parse_mysql_url(second)?;

    Some(hosts_match(first.host(), second.host()) && first.port() == second.port() && first.dbname() == second.dbname())
}

#[cfg(feature = "mysql")]
fn parse_mysql_url(url: &str) -> Option<quaint::connector::MysqlUrl> {
    quaint::connector::MysqlUrl::new(url.parse().ok()?).ok()
}

#[cfg(feature = "mssql")]
fn compare_mssql_urls(first: &str, second: &str) -> Option<bool> {
    let first = quaint::connector::MssqlUrl::new(first).ok()?;
    let second = quaint::connector::MssqlUrl::new(second).ok()?;

    Some(hosts_match(first.host(), second.host()) && first.port() == second.port() && first.dbname() == second.dbname())
}

#[cfg(feature = "sqlite")]
fn compare_sqlite_urls(first: &str, second: &str) -> Option<bool> {
    let first = quaint::connector::SqliteParams::try_from(first).ok()?;
    let second = quaint::connector::SqliteParams::try_from(second).ok()?;

    // An anonymous in-memory database is private to the connection that opened it, so it is never
    // the same database as anything else, not even as another connection to `:memory:`.
    if is_anonymous_in_memory(&first.file_path) || is_anonymous_in_memory(&second.file_path) {
        return Some(false);
    }

    Some(resolve_sqlite_path(&first.file_path) == resolve_sqlite_path(&second.file_path))
}

#[cfg(feature = "sqlite")]
fn is_anonymous_in_memory(file_path: &str) -> bool {
    file_path == ":memory:"
}

#[cfg(feature = "sqlite")]
fn resolve_sqlite_path(file_path: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(file_path);

    // `canonicalize` needs the file to exist, which is not the case for a database that is yet to
    // be created, hence the lexical fallback.
    std::fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_owned())
}

#[cfg(any(
    feature = "postgresql",
    feature = "cockroachdb",
    feature = "mysql",
    feature = "mssql"
))]
fn hosts_match(first: &str, second: &str) -> bool {
    first.eq_ignore_ascii_case(second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unparseable_connection_strings_are_compared_verbatim() {
        assert!(urls_denote_same_database(
            Flavour::Postgres,
            "not a connection string",
            "not a connection string"
        ));
        assert!(!urls_denote_same_database(
            Flavour::Postgres,
            "not a connection string",
            "not a connection string either"
        ));
        assert!(!urls_denote_same_database(
            Flavour::Postgres,
            "postgres://example.com/db",
            "not a connection string"
        ));
    }

    #[cfg(feature = "postgresql")]
    mod postgres {
        use super::*;

        fn same(first: &str, second: &str) -> bool {
            urls_denote_same_database(Flavour::Postgres, first, second)
        }

        #[test]
        fn scheme_aliases_denote_the_same_database() {
            assert!(same("postgres://example.com/mydb", "postgresql://example.com/mydb"));
        }

        #[test]
        fn host_comparison_is_case_insensitive() {
            assert!(same("postgres://Example.COM/mydb", "postgres://example.com/mydb"));
        }

        #[test]
        fn the_default_port_matches_the_same_port_spelled_out() {
            assert!(same("postgres://example.com/mydb", "postgres://example.com:5432/mydb"));
            assert!(!same("postgres://example.com/mydb", "postgres://example.com:5433/mydb"));
        }

        #[test]
        fn the_schema_selector_is_not_part_of_the_database_identity() {
            assert!(same(
                "postgres://example.com/mydb?schema=public",
                "postgres://example.com/mydb?schema=shadow"
            ));
        }

        #[test]
        fn credentials_are_not_part_of_the_database_identity() {
            assert!(same(
                "postgres://alice:secret@example.com/mydb",
                "postgres://bob:hunter2@example.com/mydb?connection_limit=1"
            ));
        }

        #[test]
        fn different_databases_on_the_same_server_are_distinct() {
            assert!(!same("postgres://example.com/mydb", "postgres://example.com/shadowdb"));
        }

        #[test]
        fn different_hosts_are_distinct() {
            assert!(!same(
                "postgres://example.com/mydb",
                "postgres://other.example.com/mydb"
            ));
        }

        #[test]
        fn prisma_postgres_connection_strings_are_compared_verbatim() {
            // The `api_key` is what tells the two databases apart, and it is not part of the
            // comparison, so the two URLs below must not be reported as the same database.
            assert!(!same(
                "prisma+postgres://localhost:51213/?api_key=first",
                "prisma+postgres://localhost:51213/?api_key=second"
            ));
            assert!(same(
                "prisma+postgres://localhost:51213/?api_key=first",
                "prisma+postgres://localhost:51213/?api_key=first"
            ));
        }
    }

    #[cfg(feature = "mysql")]
    mod mysql {
        use super::*;

        fn same(first: &str, second: &str) -> bool {
            urls_denote_same_database(Flavour::Mysql, first, second)
        }

        #[test]
        fn the_default_port_matches_the_same_port_spelled_out() {
            assert!(same("mysql://Example.com/mydb", "mysql://example.com:3306/mydb"));
            assert!(!same("mysql://example.com/mydb", "mysql://example.com:3307/mydb"));
        }

        #[test]
        fn credentials_are_not_part_of_the_database_identity() {
            assert!(same(
                "mysql://alice:secret@example.com/mydb",
                "mysql://bob:hunter2@example.com/mydb?connection_limit=1"
            ));
        }

        #[test]
        fn different_databases_on_the_same_server_are_distinct() {
            assert!(!same("mysql://example.com/mydb", "mysql://example.com/shadowdb"));
        }

        #[test]
        fn different_hosts_are_distinct() {
            assert!(!same("mysql://example.com/mydb", "mysql://other.example.com/mydb"));
        }
    }

    #[cfg(feature = "mssql")]
    mod mssql {
        use super::*;

        fn same(first: &str, second: &str) -> bool {
            urls_denote_same_database(Flavour::Sqlserver, first, second)
        }

        #[test]
        fn the_default_port_matches_the_same_port_spelled_out() {
            assert!(same(
                "sqlserver://example.com;database=mydb",
                "sqlserver://example.com:1433;database=mydb"
            ));
            assert!(!same(
                "sqlserver://example.com;database=mydb",
                "sqlserver://example.com:1434;database=mydb"
            ));
        }

        #[test]
        fn credentials_and_schema_are_not_part_of_the_database_identity() {
            assert!(same(
                "sqlserver://example.com:1433;database=mydb;user=alice;password=secret",
                "sqlserver://example.com:1433;database=mydb;user=bob;password=hunter2;schema=shadow"
            ));
        }

        #[test]
        fn different_databases_on_the_same_server_are_distinct() {
            assert!(!same(
                "sqlserver://example.com:1433;database=mydb",
                "sqlserver://example.com:1433;database=shadowdb"
            ));
        }

        #[test]
        fn different_hosts_are_distinct() {
            assert!(!same(
                "sqlserver://example.com:1433;database=mydb",
                "sqlserver://other.example.com:1433;database=mydb"
            ));
        }
    }

    #[cfg(feature = "sqlite")]
    mod sqlite {
        use super::*;

        fn same(first: &str, second: &str) -> bool {
            urls_denote_same_database(Flavour::Sqlite, first, second)
        }

        #[test]
        fn the_file_scheme_is_not_part_of_the_database_identity() {
            assert!(same("file:/tmp/prisma/dev.db", "/tmp/prisma/dev.db"));
            assert!(same("sqlite:/tmp/prisma/dev.db", "file:/tmp/prisma/dev.db"));
        }

        #[test]
        fn paths_are_compared_after_normalization() {
            assert!(same("file:/tmp/prisma/./dev.db", "file:/tmp/prisma/dev.db"));
        }

        #[test]
        fn relative_and_absolute_paths_to_the_same_file_are_the_same_database() {
            let cwd = std::env::current_dir().unwrap();
            let absolute = cwd.join("dev.db");

            assert!(same("file:dev.db", absolute.to_str().unwrap()));
        }

        #[test]
        fn different_files_are_distinct() {
            assert!(!same("file:/tmp/prisma/dev.db", "file:/tmp/prisma/shadow.db"));
        }

        #[test]
        fn anonymous_in_memory_databases_are_never_the_same_database() {
            assert!(!same(":memory:", ":memory:"));
            assert!(!same("file::memory:", ":memory:"));
        }
    }
}
