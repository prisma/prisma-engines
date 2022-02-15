use mongodb::{
    error::Result,
    options::{ClientOptions, ResolverConfig},
    Client,
};

/// A wrapper to create a new MongoDB client. Please remove me when we do not
/// need special setup anymore for this.
pub async fn create(connection_string: impl AsRef<str>) -> Result<Client> {
    let options = if cfg!(target_os = "windows") {
        ClientOptions::parse_with_resolver_config(connection_string, ResolverConfig::cloudflare()).await?
    } else {
        ClientOptions::parse(connection_string).await?
    };

    Client::with_options(options)
}
