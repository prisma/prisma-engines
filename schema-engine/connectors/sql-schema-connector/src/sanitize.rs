//! Naming a database in a user-facing message without leaking the credentials that reach it.

use connection_string::JdbcString;
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
/// names while parsing.
const CREDENTIAL_JDBC_PROPERTIES: &[&str] = &["user", "password"];

/// Renders a connection string for a user-facing message: the server, the port and the database
/// name are kept, so that the reader can tell which database is meant, while the credentials that
/// reach it — userinfo, secret-bearing query parameters, JDBC credential properties — are dropped.
pub fn sanitize_connection_string(connection_string: &str) -> String {
    let sanitized = if is_jdbc_connection_string(connection_string) {
        sanitize_jdbc_connection_string(connection_string)
    } else {
        sanitize_url(connection_string)
    };

    sanitized.unwrap_or_else(|| UNPARSEABLE_CONNECTION_STRING.to_owned())
}

/// SQL Server connection strings hold their properties in a `;`-separated list, which the `url`
/// crate happily parses as part of an opaque host, credentials and all. They are recognized by
/// their scheme instead.
fn is_jdbc_connection_string(connection_string: &str) -> bool {
    connection_string.starts_with("sqlserver:") || connection_string.starts_with("jdbc:sqlserver:")
}

fn sanitize_jdbc_connection_string(connection_string: &str) -> Option<String> {
    let with_prefix = if connection_string.starts_with("jdbc:") {
        connection_string.to_owned()
    } else {
        format!("jdbc:{connection_string}")
    };

    let mut jdbc: JdbcString = with_prefix.parse().ok()?;
    let properties = jdbc.properties_mut();

    for property in CREDENTIAL_JDBC_PROPERTIES {
        properties.remove(*property);
    }

    let sanitized = jdbc.to_string();

    Some(if connection_string.starts_with("jdbc:") {
        sanitized
    } else {
        sanitized.trim_start_matches("jdbc:").to_owned()
    })
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
            sanitize_connection_string("postgresql://alice:hunter2@db.example.com:5432/shadow"),
            "postgresql://db.example.com:5432/shadow"
        );
        assert_eq!(
            sanitize_connection_string("mysql://alice@db.example.com:3306/shadow"),
            "mysql://db.example.com:3306/shadow"
        );
    }

    #[test]
    fn secret_query_parameters_are_stripped_and_the_rest_is_kept() {
        assert_eq!(
            sanitize_connection_string("postgresql://db.example.com/shadow?schema=public&sslmode=require"),
            "postgresql://db.example.com/shadow?schema=public&sslmode=require"
        );
        assert_eq!(
            sanitize_connection_string("postgresql://db.example.com/shadow?sslpassword=hunter2&schema=public"),
            "postgresql://db.example.com/shadow?schema=public"
        );
        assert_eq!(
            sanitize_connection_string("prisma+postgres://localhost:51213/?api_key=c2VjcmV0"),
            "prisma+postgres://localhost:51213/"
        );
        assert_eq!(
            sanitize_connection_string("postgresql://db.example.com/shadow?PASSWORD=hunter2"),
            "postgresql://db.example.com/shadow"
        );
    }

    #[test]
    fn sqlite_paths_are_kept() {
        assert_eq!(
            sanitize_connection_string("file:/tmp/prisma/shadow.db"),
            "file:///tmp/prisma/shadow.db"
        );
    }

    #[test]
    fn credentials_are_stripped_from_sql_server_connection_strings() {
        let sanitized =
            sanitize_connection_string("sqlserver://db.example.com:1433;database=shadow;user=SA;password=hunter2");

        assert!(sanitized.starts_with("sqlserver://db.example.com:1433"), "{sanitized}");
        assert!(sanitized.contains("database=shadow"), "{sanitized}");
        assert!(!sanitized.contains("hunter2"), "{sanitized}");
        assert!(!sanitized.contains("SA"), "{sanitized}");
    }

    #[test]
    fn credentials_are_stripped_from_sql_server_connection_strings_without_a_port() {
        // Without a port, the `url` crate parses the whole property list as an opaque host, so this
        // is the shape that a URL-only sanitizer would echo back verbatim.
        let sanitized = sanitize_connection_string("sqlserver://db.example.com;database=shadow;password=hunter2");

        assert!(!sanitized.contains("hunter2"), "{sanitized}");
        assert!(sanitized.contains("database=shadow"), "{sanitized}");
    }

    #[test]
    fn an_unparseable_connection_string_is_not_echoed_back() {
        assert_eq!(
            sanitize_connection_string("this is not a connection string"),
            "(unparseable connection string)"
        );
    }
}
