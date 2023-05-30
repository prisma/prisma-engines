use std::borrow::Cow;

use super::*;
use prisma_models::{walkers, DefaultKind};

/// Input object type convenience wrapper function.
pub(crate) fn input_object_type<'a>(
    ident: Identifier,
    fields: impl Fn() -> Vec<InputField<'a>> + Send + Sync + 'a,
) -> InputObjectType<'a> {
    let mut object_type = init_input_object_type(ident);
    object_type.fields = Arc::new(fields);
    object_type
}

/// Input object type initializer for cases where only the name is known, and fields are computed later.
pub(crate) fn init_input_object_type<'a>(ident: Identifier) -> InputObjectType<'a> {
    InputObjectType {
        identifier: ident,
        constraints: InputObjectTypeConstraints::default(),
        fields: Arc::new(Vec::new),
        tag: None,
    }
}

/// Field convenience wrapper function.
pub(crate) fn field<'a, T>(
    name: T,
    arguments: Option<Arc<dyn Fn() -> Vec<InputField<'a>> + Send + Sync + 'a>>,
    field_type: OutputType<'a>,
    query_info: Option<QueryInfo>,
) -> OutputField<'a>
where
    T: Into<Cow<'a, str>>,
{
    OutputField {
        name: name.into(),
        arguments,
        field_type,
        query_info,
        is_nullable: false,
    }
}

/// An input field with a single possible type.
pub(crate) fn simple_input_field<'a>(
    name: impl Into<std::borrow::Cow<'a, str>>,
    field_type: InputType<'a>,
    default_value: Option<DefaultKind>,
) -> InputField<'_> {
    input_field(name, vec![field_type], default_value)
}

/// Field convenience wrapper function.
pub(crate) fn input_field<'a>(
    name: impl Into<std::borrow::Cow<'a, str>>,
    field_types: Vec<InputType<'a>>,
    default_value: Option<DefaultKind>,
) -> InputField<'a> {
    InputField::new(name.into(), field_types, default_value, true)
}

/// Appends an option of type T to a vector over T if the option is Some.
pub(crate) fn append_opt<T>(vec: &mut Vec<T>, opt: Option<T>) {
    vec.extend(opt.into_iter())
}

/// Computes a compound field name based on an index.
pub fn compound_index_field_name(index: walkers::IndexWalker<'_>) -> Cow<'_, str> {
    index
        .name()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| join_fields(index.fields().map(|f| f.name())))
}

/// Computes a compound field name based on a multi-field id.
pub fn compound_id_field_name(pk: walkers::PrimaryKeyWalker<'_>) -> Cow<'_, str> {
    pk.name()
        .map(Cow::Borrowed)
        .unwrap_or_else(|| join_fields(pk.fields().map(|f| f.name())))
}

fn join_fields<'a>(fields: impl ExactSizeIterator<Item = &'a str>) -> Cow<'a, str> {
    let mut fields = fields.peekable();
    if fields.len() == 1 {
        return Cow::Borrowed(fields.next().unwrap());
    }

    let mut name = String::with_capacity(fields.len() * 3);
    while let Some(field) = fields.next() {
        name.push_str(field);
        if fields.peek().is_some() {
            name.push('_');
        }
    }

    Cow::Owned(name)
}
