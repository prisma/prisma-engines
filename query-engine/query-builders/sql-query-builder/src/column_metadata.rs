use query_structure::{FieldArity, TypeIdentifier};

/// Helps dealing with column value conversion and possible error resolution.
#[derive(Clone, Debug, Copy)]
pub struct ColumnMetadata<'a> {
    identifier: &'a TypeIdentifier,
    name: Option<&'a str>,
    arity: FieldArity,
}

impl<'a> ColumnMetadata<'a> {
    fn new(identifier: &'a TypeIdentifier, arity: FieldArity) -> Self {
        Self {
            identifier,
            name: None,
            arity,
        }
    }

    /// If set, the errors can refer to the column holding broken data.
    fn set_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// The type of the column.
    pub fn identifier(self) -> &'a TypeIdentifier {
        self.identifier
    }

    /// The name of the column.
    pub fn name(self) -> Option<&'a str> {
        self.name
    }

    /// The arity of the column.
    pub fn arity(self) -> FieldArity {
        self.arity
    }
}

/// Create a set of metadata objects, combining column names and type
/// information.
pub fn create<'a, T>(field_names: &'a [T], idents: &'a [(TypeIdentifier, FieldArity)]) -> Vec<ColumnMetadata<'a>>
where
    T: AsRef<str>,
{
    assert_eq!(field_names.len(), idents.len());

    idents
        .iter()
        .zip(field_names.iter())
        .map(|((identifier, arity), name)| ColumnMetadata::new(identifier, *arity).set_name(name.as_ref()))
        .collect()
}

/// Create a set of metadata objects.
pub fn create_anonymous(idents: &[(TypeIdentifier, FieldArity)]) -> Vec<ColumnMetadata<'_>> {
    idents
        .iter()
        .map(|(identifier, arity)| ColumnMetadata::new(identifier, *arity))
        .collect()
}
