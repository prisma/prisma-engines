use crate::StringFromEnvVar;
use datamodel_connector::{Connector, ConnectorCapabilities};
use std::{collections::BTreeMap, path::Path};
use url::Url;

/// a `datasource` from the prisma schema.
pub struct Datasource {
    pub name: String,
    /// all providers that were specified
    pub provider: Vec<String>,
    /// the provider that was selected as active from all specified providers
    pub active_provider: String,
    pub url: StringFromEnvVar,
    pub documentation: Option<String>,
    /// a connector representing the intersection of all providers specified
    pub combined_connector: Box<dyn Connector>,
    /// the connector of the active provider
    pub active_connector: Box<dyn Connector>,
    /// An optional user-defined shadow database URL.
    pub shadow_database_url: Option<StringFromEnvVar>,
}

impl std::fmt::Debug for Datasource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Datasource")
            .field("name", &self.name)
            .field("provider", &self.provider)
            .field("active_provider", &self.active_provider)
            .field("url", &self.url)
            .field("documentation", &self.documentation)
            .field("active_connector", &&"...")
            .field("shadow_database_url", &self.shadow_database_url)
            .finish()
    }
}

impl Datasource {
    pub fn url(&self) -> &StringFromEnvVar {
        &self.url
    }

    pub fn capabilities(&self) -> ConnectorCapabilities {
        let capabilities = self.active_connector.capabilities().clone();
        ConnectorCapabilities::new(capabilities)
    }

    /// JavaScript wants to set the CWD, but can't due to changing it for itself
    /// too. This method is an ugly hack to convert all relative paths in the
    /// connection string to include a given config directory as its prefix, but
    /// only if the path is relative.
    ///
    /// The other option would've been to just point in our docs that a path
    /// in a connection string is relative only from the caller point of view.
    ///
    /// Now, there is a promise to change this behavior to follow how everybody
    /// else is doing it, but I wouldn't hold my breath.
    ///
    /// P.S. Don't forget to add new parameters here if needed!
    pub fn set_config_dir(&mut self, config_dir: &Path) {
        let set_root = |path: &str| {
            let path = Path::new(path);

            if path.is_relative() {
                Some(config_dir.join(&path).to_str().map(ToString::to_string).unwrap())
            } else {
                None
            }
        };

        match self.active_provider.as_str() {
            "sqlserver" => (),
            "sqlite" => {
                if let Some(path) = set_root(&self.url.value.trim_start_matches("file:")) {
                    self.url.value = format!("file:{}", path);
                };
            }
            _ => {
                let mut url = Url::parse(&self.url.value).unwrap();

                let mut params: BTreeMap<String, String> =
                    url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect();

                url.query_pairs_mut().clear();

                if let Some(path) = params.get("sslcert").map(|s| s.as_str()).and_then(set_root) {
                    params.insert("sslcert".into(), path);
                }

                if let Some(path) = params.get("sslidentity").map(|s| s.as_str()).and_then(set_root) {
                    params.insert("sslidentity".into(), path);
                }

                for (k, v) in params.into_iter() {
                    url.query_pairs_mut().append_pair(&k, &v);
                }

                self.url.value = url.to_string();
            }
        }
    }
}
