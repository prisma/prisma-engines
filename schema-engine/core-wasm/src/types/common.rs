/// The path to a live database taken as input. For flexibility, this can be Prisma schemas as strings, or only the
/// connection string. See variants.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub enum DatasourceParam {
    /// A container that holds multiple Prisma schema files.
    Schema(SchemasContainer),

    /// An object with a `url` field.
    ConnectionString(UrlContainer),
}

/// A container that holds the path and the content of a Prisma schema file.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SchemaContainer {
    /// The content of the Prisma schema file.
    content: String,

    /// The file name of the Prisma schema file.
    path: String,
}

/// A list of Prisma schema files with a config directory.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct SchemasWithConfigDir {
    /// A list of Prisma schema files.
    files: Vec<SchemaContainer>,

    /// An optional directory containing the config files such as SSL certificates
    config_dir: String,
}

/// An object with a `url` field.
#[derive(Debug, Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(from_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct UrlContainer {
    url: String,
}
