use crate::{UdtId, UserDefinedType, Walker};

/// Traverse a user-defined type
pub type UserDefinedTypeWalker<'a> = Walker<'a, UdtId>;

impl<'a> UserDefinedTypeWalker<'a> {
    /// The name of the type
    pub fn name(self) -> &'a str {
        &self.get().name
    }

    /// The SQL definition of the type
    pub fn definition(self) -> Option<&'a str> {
        self.get().definition.as_deref()
    }

    /// The namespace of the type
    pub fn namespace(self) -> Option<&'a str> {
        self.schema
            .namespaces
            .get_index(self.get().namespace_id.0 as usize)
            .map(|s| s.as_str())
    }

    fn get(self) -> &'a UserDefinedType {
        &self.schema.user_defined_types[self.id.0 as usize]
    }
}
