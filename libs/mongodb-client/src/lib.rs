use mongodb::{
    error::Result,
    options::{ClientOptions, DriverInfo, ResolverConfig},
    Client,
};

/// A wrapper to create a new MongoDB client. Please remove me when we do not
/// need special setup anymore for this.
pub async fn create(connection_string: impl AsRef<str>) -> Result<Client> {
    let mut options = if cfg!(target_os = "windows") {
        ClientOptions::parse_with_resolver_config(connection_string, ResolverConfig::cloudflare()).await?
    } else {
        ClientOptions::parse(connection_string).await?
    };

    options.driver_info = Some(DriverInfo::builder().name("Prisma").build());

    Client::with_options(options)
}
