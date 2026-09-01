//! Naming a database in a user-facing message without leaking the credentials that reach it.

use connection_string::JdbcString;
use psl::datamodel_connector::Flavour;
use url::Url;

/// How a shadow database reached through a driver adapter is named: there is no connection string
/// to show.
pub const DRIVER_ADAPTER_SHADOW_DATABASE: &str = "(driver adapter shadow database)";

/// Stands in for a connection string that could not be parsed, which must not be echoed back
/// verbatim: what cannot be parsed cannot be stripped of its secrets either.
const UNPARSEABLE_CONNECTION_STRING: &str = "(unparseable connection string)";

/// The query string parameters that carry a secret rather than a connection setting.
const SECRET_QUERY_PARAMS: &[&str] = &["api_key", "password", "sslpassword"];

/// The JDBC properties that carry credentials. The `connection-string` crate lower-cases property
/// names while parsing, so these match however the user spelled them.
const CREDENTIAL_JDBC_PROPERTIES: &[&str] = &["user", "password"];

const JDBC_PREFIX: &str = "jdbc:";

/// Renders a connection string for a user-facing message: the server, the port and the database
/// name are kept, so that the reader can tell which database is meant, while the credentials that
/// reach it — userinfo, secret-bearing query parameters, JDBC credential properties — are dropped.
///
/// The flavour decides how the connection string is read, because the syntaxes are not mutually
/// exclusive: a SQL Server connection string is a valid URL whose entire property list, password
/// included, parses as an opaque host, and a SQLite connection string may be a bare file path that
/// is no URL at all. A connection string that cannot be read as its flavour renders as a
/// placeholder instead of being echoed back with its secrets in unknown positions.
pub fn sanitize_connection_string(flavour: Flavour, connection_string: &str) -> String {
    let sanitized = match flavour {
        Flavour::Sqlserver => sanitize_jdbc_connection_string(connection_string),
        Flavour::Sqlite => Some(sqlite_file_path(connection_string)),
        Flavour::Postgres | Flavour::Cockroach | Flavour::Mysql | Flavour::Mongo => sanitize_url(connection_string),
    };

    sanitized.unwrap_or_else(|| UNPARSEABLE_CONNECTION_STRING.to_owned())
}

fn sanitize_jdbc_connection_string(connection_string: &str) -> Option<String> {
    let had_prefix = connection_string
        .get(..JDBC_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(JDBC_PREFIX));

    // `JdbcString` matches the leading `jdbc` literal case-sensitively, and quaint prepends it to
    // the connection strings that come without it.
    let with_prefix = if had_prefix {
        format!("{JDBC_PREFIX}{}", &connection_string[JDBC_PREFIX.len()..])
    } else {
        format!("{JDBC_PREFIX}{connection_string}")
    };

    let mut jdbc: JdbcString = with_prefix.parse().ok()?;
    let properties = jdbc.properties_mut();

    for property in CREDENTIAL_JDBC_PROPERTIES {
        properties.remove(*property);
    }

    let sanitized = jdbc.to_string();

    Some(if had_prefix {
        sanitized
    } else {
        sanitized[JDBC_PREFIX.len()..].to_owned()
    })
}

/// A SQLite connection string holds a file path, which carries no credentials to strip. The scheme
/// and the connection parameters are trimmed the way quaint trims them when it opens the file.
fn sqlite_file_path(connection_string: &str) -> String {
    let path = connection_string
        .strip_prefix("file:")
        .or_else(|| connection_string.strip_prefix("sqlite:"))
        .unwrap_or(connection_string);

    path.split('?').next().unwrap_or(path).to_owned()
}

fn sanitize_url(connection_string: &str) -> Option<String> {
    let mut url: Url = connection_string.parse().ok()?;

    if !url.username().is_empty() {
        url.set_username("").ok()?;
    }

    if url.password().is_some() {
        url.set_password(None).ok()?;
    }

    let params: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| {
            !SECRET_QUERY_PARAMS
                .iter()
                .any(|secret| name.eq_ignore_ascii_case(secret))
        })
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();

    if params.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(params);
    }

    Some(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_stripped_from_urls() {
        assert_eq!(
            sanitize_connection_string(
                Flavour::Postgres,
                "postgresql://alice:hunter2@db.example.com:5432/shadow"
            ),
            "postgresql://db.example.com:5432/shadow"
        );
        assert_eq!(
            sanitize_connection_string(Flavour::Mysql, "mysql://alice@db.example.com:3306/shadow"),
            "mysql://db.example.com:3306/shadow"
        );
    }

    #[test]
    fn the_case_of_a_url_scheme_does_not_hide_credentials() {
        let sanitized = sanitize_connection_string(
            Flavour::Postgres,
            "POSTGRESQL://alice:hunter2@db.example.com:5432/shadow",
        );

        assert!(!sanitized.contains("hunter2"), "{sanitized}");
        assert!(sanitized.contains("db.example.com:5432/shadow"), "{sanitized}");
    }

    #[test]
    fn secret_query_parameters_are_stripped_and_the_rest_is_kept() {
        assert_eq!(
            sanitize_connection_string(
                Flavour::Postgres,
                "postgresql://db.example.com/shadow?schema=public&sslmode=require"
            ),
            "postgresql://db.example.com/shadow?schema=public&sslmode=require"
        );
        assert_eq!(
            sanitize_connection_string(
                Flavour::Postgres,
                "postgresql://db.example.com/shadow?sslpassword=hunter2&schema=public"
            ),
            "postgresql://db.example.com/shadow?schema=public"
        );
        assert_eq!(
            sanitize_connection_string(Flavour::Postgres, "prisma+postgres://localhost:51213/?api_key=c2VjcmV0"),
            "prisma+postgres://localhost:51213/"
        );
        assert_eq!(
            sanitize_connection_string(Flavour::Postgres, "postgresql://db.example.com/shadow?PASSWORD=hunter2"),
            "postgresql://db.example.com/shadow"
        );
    }

    #[test]
    fn sqlite_paths_are_rendered_as_paths() {
        assert_eq!(
            sanitize_connection_string(Flavour::Sqlite, "file:/tmp/prisma/shadow.db"),
            "/tmp/prisma/shadow.db"
        );
        assert_eq!(
            sanitize_connection_string(Flavour::Sqlite, "sqlite:/tmp/prisma/shadow.db"),
            "/tmp/prisma/shadow.db"
        );
        assert_eq!(
            sanitize_connection_string(Flavour::Sqlite, "file:./shadow.db?connection_limit=1"),
            "./shadow.db"
        );
    }

    #[test]
    fn a_bare_sqlite_path_is_rendered_as_it_is() {
        assert_eq!(sanitize_connection_string(Flavour::Sqlite, "shadow.db"), "shadow.db");
    }

    #[test]
    fn credentials_are_stripped_from_sql_server_connection_strings() {
        let sanitized = sanitize_connection_string(
            Flavour::Sqlserver,
            "sqlserver://db.example.com:1433;database=shadow;user=SA;password=hunter2",
        );

        assert!(sanitized.starts_with("sqlserver://db.example.com:1433"), "{sanitized}");
        assert!(sanitized.contains("database=shadow"), "{sanitized}");
        assert!(!sanitized.contains("hunter2"), "{sanitized}");
        assert!(!sanitized.contains("SA"), "{sanitized}");
    }

    #[test]
    fn credentials_are_stripped_from_sql_server_connection_strings_without_a_port() {
        // Without a port, the `url` crate parses the whole property list as an opaque host, so this
        // is the shape that a URL sanitizer would echo back verbatim.
        let sanitized = sanitize_connection_string(
            Flavour::Sqlserver,
            "sqlserver://db.example.com;database=shadow;password=hunter2",
        );

        assert!(!sanitized.contains("hunter2"), "{sanitized}");
        assert!(sanitized.contains("database=shadow"), "{sanitized}");
    }

    #[test]
    fn the_case_of_a_sql_server_scheme_does_not_hide_credentials() {
        for connection_string in [
            "SQLSERVER://db.example.com;database=shadow;password=hunter2",
            "SqlServer://db.example.com:1433;database=shadow;PASSWORD=hunter2",
            "JDBC:sqlserver://db.example.com:1433;database=shadow;password=hunter2",
            "jdbc:SQLSERVER://db.example.com:1433;database=shadow;user=SA;password=hunter2",
        ] {
            let sanitized = sanitize_connection_string(Flavour::Sqlserver, connection_string);

            assert!(!sanitized.contains("hunter2"), "{connection_string}: {sanitized}");
            assert!(
                sanitized.contains("database=shadow"),
                "{connection_string}: {sanitized}"
            );
        }
    }

    #[test]
    fn an_unparseable_connection_string_is_not_echoed_back() {
        assert_eq!(
            sanitize_connection_string(Flavour::Postgres, "this is not a connection string"),
            "(unparseable connection string)"
        );
        assert_eq!(
            sanitize_connection_string(Flavour::Sqlserver, "this is not a connection string"),
            "(unparseable connection string)"
        );
    }
}
