use super::PostgresSchemaExt;

#[derive(Debug, Clone)]
pub struct DatabaseExtension {
    pub name: String,
    pub schema: String,
    pub version: String,
    pub relocatable: bool,
}

/// The identifier for an extension in a Postgres database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionId(pub(crate) u32);

/// Traverse an extension
#[derive(Clone, Copy)]
pub struct ExtensionWalker<'a> {
    pub id: ExtensionId,
    pub(super) schema_ext: &'a PostgresSchemaExt,
}

impl<'a> ExtensionWalker<'a> {
    /// The name of the extension.
    pub fn name(self) -> &'a str {
        &self.extension().name
    }

    /// The schema where the extension data is located.
    pub fn schema(self) -> &'a str {
        &self.extension().schema
    }

    /// The version of the extension.
    pub fn version(self) -> &'a str {
        &self.extension().version
    }

    /// Can the extension data be relocated to a different schema.
    pub fn relocatable(self) -> bool {
        self.extension().relocatable
    }

    fn extension(self) -> &'a DatabaseExtension {
        &self.schema_ext.extensions[self.id.0 as usize]
    }
}
