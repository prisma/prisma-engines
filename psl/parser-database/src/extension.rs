/// A trait for looking up extension types.
pub trait ExtensionTypes {
    /// Look up an extension type by its name.
    /// Returns `None` if the extension type is not known.
    fn extension_type_by_name(&self, name: &str) -> Option<ExtensionTypeId>;
}

/// An identifier for an extension type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExtensionTypeId(usize);

impl From<usize> for ExtensionTypeId {
    fn from(value: usize) -> Self {
        ExtensionTypeId(value)
    }
}

/// An empty implementation of `ExtensionTypes` that knows no extension types.
#[derive(Debug, Default)]
pub struct NoExtensions;

impl ExtensionTypes for NoExtensions {
    fn extension_type_by_name(&self, _name: &str) -> Option<ExtensionTypeId> {
        None
    }
}
