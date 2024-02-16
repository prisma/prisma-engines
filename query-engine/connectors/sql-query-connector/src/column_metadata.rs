use query_structure::{
    FieldArity, FieldSelection, GroupedSelectedField, GroupedVirtualSelection, RelationSelection, TypeIdentifier,
};

#[derive(Clone, Debug)]
pub enum MetadataFieldKind<'a> {
    Scalar,
    Relation(&'a RelationSelection),
    Virtual(GroupedVirtualSelection<'a>),
}

/// Helps dealing with column value conversion and possible error resolution.
#[derive(Clone, Debug)]
pub(crate) struct ColumnMetadata<'a> {
    identifier: TypeIdentifier,
    name: Option<&'a str>,
    arity: FieldArity,
    kind: MetadataFieldKind<'a>,
}

impl<'a> ColumnMetadata<'a> {
    fn new(identifier: TypeIdentifier, arity: FieldArity, kind: MetadataFieldKind<'a>) -> Self {
        Self {
            identifier,
            name: None,
            arity,
            kind,
        }
    }

    /// If set, the errors can refer to the column holding broken data.
    fn set_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// The type of the column.
    pub fn identifier(&self) -> TypeIdentifier {
        self.identifier
    }

    /// The name of the column.
    pub fn name(&self) -> Option<&'a str> {
        self.name
    }

    /// The arity of the column.
    pub fn arity(&self) -> FieldArity {
        self.arity
    }

    pub(crate) fn kind(&self) -> &MetadataFieldKind<'_> {
        &self.kind
    }
}

/// Create a set of metadata objects, combining column names and type
/// information.
pub(crate) fn create<'a, T>(field_names: &'a [T], idents: &'a [(TypeIdentifier, FieldArity)]) -> Vec<ColumnMetadata<'a>>
where
    T: AsRef<str>,
{
    assert_eq!(field_names.len(), idents.len());

    idents
        .iter()
        .zip(field_names.iter())
        .map(|((identifier, arity), name)| {
            ColumnMetadata::new(*identifier, *arity, MetadataFieldKind::Scalar).set_name(name.as_ref())
        })
        .collect()
}

pub(crate) fn create_from_selection_for_json<'a, T>(
    selection: &'a FieldSelection,
    field_names: &'a [T],
) -> Vec<ColumnMetadata<'a>>
where
    T: AsRef<str>,
{
    selection
        .grouped_selections()
        .zip(field_names.iter())
        .map(|(field, name)| {
            let (type_identifier, arity) = field.type_identifier_with_arity_for_json();

            let kind = match field {
                GroupedSelectedField::Scalar(_) => MetadataFieldKind::Scalar,
                GroupedSelectedField::Relation(rs) => MetadataFieldKind::Relation(rs),
                GroupedSelectedField::Virtual(vs) => MetadataFieldKind::Virtual(vs),
            };

            ColumnMetadata::new(type_identifier, arity, kind).set_name(name.as_ref())
        })
        .collect()
}

/// Create a set of metadata objects.
pub(crate) fn create_anonymous(idents: &[(TypeIdentifier, FieldArity)]) -> Vec<ColumnMetadata<'_>> {
    idents
        .iter()
        .map(|(identifier, arity)| ColumnMetadata::new(*identifier, *arity, MetadataFieldKind::Scalar))
        .collect()
}
