/// A trait for looking up extension types.
pub trait ExtensionTypes: Sync {
    /// Look up an extension type by its name.
    /// Returns `None` if the extension type is not known.
    fn get_by_prisma_name(&self, name: &str) -> Option<ExtensionTypeId>;

    /// Look up an extension type by its database name and optional type modifiers.
    /// Returns `None` if the extension type is not known.
    fn get_by_db_name_and_modifiers(&self, name: &str, modifiers: Option<&[String]>) -> Option<ExtensionTypeEntry<'_>>;

    /// Enumerate all known extension types.
    fn enumerate(&self) -> Box<dyn Iterator<Item = ExtensionTypeEntry<'_>> + '_>;
}

/// An entry describing an extension type.
#[derive(Debug)]
pub struct ExtensionTypeEntry<'a> {
    /// The identifier of the extension type.
    pub id: ExtensionTypeId,
    /// The name of the extension type in the Prisma schema.
    pub prisma_name: &'a str,
    /// The name of the extension type in the database.
    pub db_name: &'a str,
    /// The namespace (or schema) of the extension type in the database.
    pub db_namespace: Option<&'a str>,
    /// The expected modifiers for the database type, if this type requires them to have
    /// specific values.
    pub db_type_modifiers: Option<&'a [String]>,
    /// The number of arguments that must be provided when using this extension type.
    pub number_of_args: usize,
}

/// An identifier for an extension type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExtensionTypeId(usize);

impl From<usize> for ExtensionTypeId {
    fn from(value: usize) -> Self {
        ExtensionTypeId(value)
    }
}

impl From<ExtensionTypeId> for usize {
    fn from(value: ExtensionTypeId) -> Self {
        value.0
    }
}

/// An empty implementation of `ExtensionTypes` that knows no extension types.
#[derive(Debug, Default)]
pub struct NoExtensionTypes;

impl ExtensionTypes for NoExtensionTypes {
    fn get_by_prisma_name(&self, _name: &str) -> Option<ExtensionTypeId> {
        None
    }

    fn get_by_db_name_and_modifiers(
        &self,
        _name: &str,
        _modifiers: Option<&[String]>,
    ) -> Option<ExtensionTypeEntry<'_>> {
        None
    }

    fn enumerate(&self) -> Box<dyn Iterator<Item = ExtensionTypeEntry<'_>> + '_> {
        Box::new(std::iter::empty())
    }
}
