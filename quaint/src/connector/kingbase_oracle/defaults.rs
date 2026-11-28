/// KingbaseES defaults to the `test` maintenance database.
pub const DEFAULT_KINGBASE_ORACLE_DB: &str = "test";

/// The tested Oracle-compatible installation resolves unqualified objects in
/// `public` unless the connection URL selects another schema.
pub const DEFAULT_KINGBASE_ORACLE_SCHEMA: &str = "public";

/// KingbaseES default TCP port. Compatibility mode does not change it.
pub const DEFAULT_KINGBASE_ORACLE_PORT: u16 = 54321;
