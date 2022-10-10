use parser_database::ast;

/// Postgres-specific properties in the datasource block.
pub struct PostgresDatasourceProperties {
    extensions: Option<PostgresExtensions>,
}

impl PostgresDatasourceProperties {
    pub fn new(extensions: Option<PostgresExtensions>) -> Self {
        Self { extensions }
    }

    /// Database extensions.
    pub fn extensions(&self) -> Option<&PostgresExtensions> {
        self.extensions.as_ref()
    }
}

/// An extension defined in the extensions array of the datasource.
///
/// ```ignore
/// datasource db {
///   extensions = [postgis, foobar]
///   //            ^^^^^^^
/// }
/// ```
pub struct PostgresExtension {
    name: String,
    span: ast::Span,
    schema: Option<String>,
    version: Option<String>,
    db_name: Option<String>,
}

impl PostgresExtension {
    pub fn new(
        name: String,
        span: ast::Span,
        schema: Option<String>,
        version: Option<String>,
        db_name: Option<String>,
    ) -> Self {
        Self {
            name,
            span,
            schema,
            version,
            db_name,
        }
    }

    /// The name of the extension in the datasource.
    ///
    /// ```ignore
    /// extensions = [bar]
    /// //            ^^^ this
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How the extension is named in the database.
    ///
    /// Either:
    ///
    /// ```ignore
    /// extensions = [bar(map: "foo")]
    /// //                     ^^^^^ this
    /// ```
    ///
    /// or if not defined:
    ///
    /// ```ignore
    /// extensions = [bar]
    /// //            ^^^ this
    /// ```
    pub fn db_name(&self) -> Option<&str> {
        self.db_name.as_deref()
    }

    /// The span of the extension definition in the datamodel.
    pub fn span(&self) -> ast::Span {
        self.span
    }

    /// The schema where the extension tables are stored.
    ///
    /// ```ignore
    /// extensions = [postgis(schema: "public")]
    /// //                            ^^^^^^^^ this
    /// ```
    pub fn schema(&self) -> Option<&str> {
        self.schema.as_deref()
    }

    /// The version of the extension to be used in the database.
    ///
    /// ```ignore
    /// extensions = [postgis(version: "2.1")]
    /// //                             ^^^^^ this
    /// ```
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

/// The extensions defined in the extensions array of the datrasource.
///
/// ```ignore
/// datasource db {
///   extensions = [postgis, foobar]
///   //           ^^^^^^^^^^^^^^^^^
/// }
/// ```
pub struct PostgresExtensions {
    pub(crate) extensions: Vec<PostgresExtension>,
    pub(crate) span: ast::Span,
}

impl PostgresExtensions {
    pub fn new(extensions: Vec<PostgresExtension>, span: ast::Span) -> Self {
        Self { extensions, span }
    }

    /// The span of the extensions in the datamodel.
    pub fn span(&self) -> ast::Span {
        self.span
    }

    /// The extension definitions.
    pub fn extensions(&self) -> &[PostgresExtension] {
        &self.extensions
    }
}
