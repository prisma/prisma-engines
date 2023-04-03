use super::*;
use once_cell::sync::OnceCell;
use prisma_models::{ast, walkers, DefaultKind};

/// Object type convenience wrapper function.
pub fn object_type(ident: Identifier, fields: Vec<OutputField>, model: Option<ast::ModelId>) -> ObjectType {
    let object_type = ObjectType::new(ident, model);

    object_type.set_fields(fields);
    object_type
}

/// Input object type initializer for cases where only the name is known, and fields are computed later.
pub fn init_input_object_type(ident: Identifier) -> InputObjectType {
    InputObjectType {
        identifier: ident,
        constraints: InputObjectTypeConstraints::default(),
        tag: None,
    }
}

/// Field convenience wrapper function.
pub fn field<T>(
    name: T,
    arguments: Vec<InputField>,
    field_type: OutputType,
    query_info: Option<QueryInfo>,
) -> OutputField
where
    T: Into<String>,
{
    OutputField {
        name: name.into(),
        arguments,
        field_type,
        query_info,
        is_nullable: false,
    }
}

/// Field convenience wrapper function.
pub(crate) fn input_field<T, S>(
    ctx: &mut BuilderContext<'_>,
    name: T,
    field_types: S,
    default_value: Option<DefaultKind>,
) -> InputField
where
    T: Into<String>,
    S: IntoIterator<Item = InputType>,
{
    let mut input_field = InputField::new(name.into(), default_value, true);
    for field_type in field_types {
        input_field.push_type(field_type, &mut ctx.db);
    }
    input_field
}

/// Appends an option of type T to a vector over T if the option is Some.
pub(crate) fn append_opt<T>(vec: &mut Vec<T>, opt: Option<T>) {
    vec.extend(opt.into_iter())
}

/// Computes a compound field name based on an index.
pub fn compound_index_field_name(index: &walkers::IndexWalker<'_>) -> String {
    index.name().map(ToOwned::to_owned).unwrap_or_else(|| {
        let field_names: Vec<&str> = index.fields().map(|sf| sf.name()).collect();

        field_names.join("_")
    })
}

/// Computes a compound field name based on a multi-field id.
pub fn compound_id_field_name(pk: walkers::PrimaryKeyWalker<'_>) -> String {
    pk.name()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| pk.fields().map(|sf| sf.name()).collect::<Vec<_>>().join("_"))
}
