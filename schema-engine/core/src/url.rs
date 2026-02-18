use connection_string::JdbcString;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::BTreeMap, path::Path};

use psl::datamodel_connector::Flavour;

use crate::core_error::CoreError;

/// Raw datasource URLs as provided by JavaScript overrides or CLI payloads.
/// These values have not gone through any validation yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasourceUrls {
    /// Direct URL to the database.
    pub url: Option<String>,
    /// The URL to a live shadow database, if Prisma should use it instead of creating one.
    pub shadow_database_url: Option<String>,
}

/// Datasource URLs that have passed validation and are safe to consume by the schema engine.
#[derive(Debug, Clone)]
pub struct ValidatedDatasourceUrls {
    url: Option<String>,
    shadow_database_url: Option<String>,
}

/// Errors produced while validating datasource URLs supplied from configuration.
#[derive(thiserror::Error, Debug)]
pub enum DatasourceError {
    #[error("`datasource.{_0}` in `prisma.config.ts` must not be an empty string.")]
    Empty(&'static str),

    #[error(
        "`datasource.{_0}` in `prisma.config.ts` must be a direct URL that points directly to the database. Using `prisma:` in the URL scheme is not allowed."
    )]
    AccelerateUrl(&'static str),

    #[error("`datasource.{_0}` in `prisma.config.ts` is invalid: {_1}")]
    ConnectorError(&'static str, String),
}

impl DatasourceUrls {
    /// Creates a `DatasourceUrls` instance containing only a primary URL.
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            shadow_database_url: None,
        }
    }

    /// Creates a `DatasourceUrls` instance with both primary and shadow database URLs.
    pub fn from_url_and_shadow_database_url(url: impl Into<String>, shadow_database_url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            shadow_database_url: Some(shadow_database_url.into()),
        }
    }

    /// Validates the URLs.
    pub fn validate(
        &self,
        connector: &dyn psl::datamodel_connector::Connector,
    ) -> Result<ValidatedDatasourceUrls, DatasourceError> {
        if let Some(url) = &self.url {
            validate_datasource_url(url, connector, "url")?;
        }

        if let Some(shadow_database_url) = &self.shadow_database_url {
            validate_datasource_url(shadow_database_url, connector, "shadowDatabaseUrl")?;
        }

        Ok(ValidatedDatasourceUrls {
            url: self.url.clone(),
            shadow_database_url: self.shadow_database_url.clone(),
        })
    }
}

fn validate_datasource_url(
    url: &str,
    connector: &dyn psl::datamodel_connector::Connector,
    name: &'static str,
) -> Result<(), DatasourceError> {
    if url.is_empty() {
        return Err(DatasourceError::Empty(name));
    }

    if url.starts_with("prisma://") {
        return Err(DatasourceError::AccelerateUrl(name));
    }

    connector
        .validate_url(url)
        .map_err(|msg| DatasourceError::ConnectorError(name, msg))
}

impl From<ValidatedDatasourceUrls> for DatasourceUrls {
    fn from(urls: ValidatedDatasourceUrls) -> Self {
        Self {
            url: urls.url,
            shadow_database_url: urls.shadow_database_url,
        }
    }
}

impl From<&DatasourceError> for CoreError {
    fn from(error: &DatasourceError) -> Self {
        CoreError::new_invalid_datasource_error(error)
    }
}

impl From<DatasourceError> for CoreError {
    fn from(error: DatasourceError) -> Self {
        Self::from(&error)
    }
}

impl ValidatedDatasourceUrls {
    /// Returns the validated primary datasource URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the validated shadow database URL, if any.
    pub fn shadow_database_url(&self) -> Option<&str> {
        self.shadow_database_url.as_deref()
    }

    /// Resolves relative paths in the URL against the provided configuration directory.
    pub fn url_with_config_dir(&self, flavour: Flavour, config_dir: &Path) -> Option<Cow<'_, str>> {
        self.url.as_deref().map(|url| set_config_dir(flavour, config_dir, url))
    }

    /// Returns both URLs with relative file paths rewritten relative to the configuration directory.
    pub fn with_config_dir(&self, flavour: Flavour, config_dir: &Path) -> DatasourceUrlsWithConfigDir<'_> {
        DatasourceUrlsWithConfigDir {
            url: self.url_with_config_dir(flavour, config_dir),
            shadow_database_url: self
                .shadow_database_url
                .as_deref()
                .map(|url| set_config_dir(flavour, config_dir, url)),
        }
    }
}

/// Datasource URLs with relative paths resolved against the configuration directory.
#[derive(Debug, Clone)]
pub struct DatasourceUrlsWithConfigDir<'a> {
    url: Option<Cow<'a, str>>,
    shadow_database_url: Option<Cow<'a, str>>,
}

impl DatasourceUrlsWithConfigDir<'_> {
    /// Returns the primary datasource URL, with resolved paths if needed.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the shadow database URL, with resolved paths if present.
    pub fn shadow_database_url(&self) -> Option<&str> {
        self.shadow_database_url.as_deref()
    }
}

fn set_config_dir<'a>(flavour: Flavour, config_dir: &Path, url: &'a str) -> Cow<'a, str> {
    match flavour {
        Flavour::Sqlserver => set_config_dir_mssql(config_dir, url),
        Flavour::Sqlite => set_config_dir_sqlite(config_dir, url),
        _ => set_config_dir_default(config_dir, url),
    }
}

fn set_config_dir_default<'a>(config_dir: &Path, url: &'a str) -> Cow<'a, str> {
    let set_root = |path: &str| {
        let path = Path::new(path);

        if path.is_relative() {
            Some(config_dir.join(path).to_str().map(ToString::to_string).unwrap())
        } else {
            None
        }
    };

    let mut url = match url::Url::parse(url) {
        Ok(url) => url,
        Err(_) => return Cow::from(url), // bail
    };

    let mut params: BTreeMap<String, String> = url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    url.query_pairs_mut().clear();

    // Only for PostgreSQL + MySQL
    if let Some(path) = params.get("sslcert").map(|s| s.as_str()).and_then(set_root) {
        params.insert("sslcert".into(), path);
    }

    // Only for PostgreSQL + MySQL
    if let Some(path) = params.get("sslidentity").map(|s| s.as_str()).and_then(set_root) {
        params.insert("sslidentity".into(), path);
    }

    // Only for MongoDB
    if let Some(path) = params.get("tlsCAFile").map(|s| s.as_str()).and_then(set_root) {
        params.insert("tlsCAFile".into(), path);
    }

    for (k, v) in params.into_iter() {
        url.query_pairs_mut().append_pair(&k, &v);
    }

    url.to_string().into()
}

fn set_config_dir_mssql<'a>(config_dir: &Path, url: &'a str) -> Cow<'a, str> {
    let mut jdbc: JdbcString = match format!("jdbc:{url}").parse() {
        Ok(jdbc) => jdbc,
        _ => return Cow::from(url),
    };

    let set_root = |path: String| {
        let path = Path::new(&path);

        if path.is_relative() {
            Some(config_dir.join(path).to_str().map(ToString::to_string).unwrap())
        } else {
            Some(path.to_str().unwrap().to_string())
        }
    };

    let props = jdbc.properties_mut();

    let cert_path = props.remove("trustservercertificateca").and_then(set_root);

    if let Some(path) = cert_path {
        props.insert("trustServerCertificateCA".to_owned(), path);
    }

    let final_connection_string = format!("{jdbc}").replace("jdbc:sqlserver://", "sqlserver://");

    Cow::Owned(final_connection_string)
}

fn set_config_dir_sqlite<'a>(config_dir: &Path, url: &'a str) -> Cow<'a, str> {
    let set_root = |path: &str| {
        let path = Path::new(path);

        if path.is_relative() {
            Some(config_dir.join(path).to_str().map(ToString::to_string).unwrap())
        } else {
            None
        }
    };

    if let Some(path) = set_root(url.trim_start_matches("file:")) {
        return Cow::Owned(format!("file:{path}"));
    };

    Cow::Borrowed(url)
}
