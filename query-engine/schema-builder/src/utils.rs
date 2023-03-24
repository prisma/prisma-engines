use super::*;
use once_cell::sync::OnceCell;
use prisma_models::pk::PrimaryKey;
use prisma_models::{dml, walkers, ModelRef};
use std::{fmt, sync::Arc};

/// Object type convenience wrapper function.
pub fn object_type(ident: Identifier, fields: Vec<OutputField>, model: Option<ModelRef>) -> ObjectType {
    let object_type = ObjectType::new(ident, model);

    object_type.set_fields(fields);
    object_type
}

/// Input object type convenience wrapper function.
pub fn input_object_type(ident: Identifier, fields: Vec<InputField>) -> InputObjectType {
    let object_type = init_input_object_type(ident);

    object_type.set_fields(fields);
    object_type
}

/// Input object type initializer for cases where only the name is known, and fields are computed later.
pub fn init_input_object_type(ident: Identifier) -> InputObjectType {
    InputObjectType {
        identifier: ident,
        constraints: InputObjectTypeConstraints::default(),
        fields: OnceCell::new(),
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
        arguments: arguments.into_iter().map(Arc::new).collect(),
        field_type: Arc::new(field_type),
        query_info,
        is_nullable: false,
        deprecation: None,
    }
}

/// Field convenience wrapper function.
pub(crate) fn input_field<T, S>(
    ctx: &mut BuilderContext,
    name: T,
    field_types: S,
    default_value: Option<dml::DefaultKind>,
) -> InputField
where
    T: Into<String>,
    S: IntoIterator<Item = InputType>,
{
    let mut input_field = InputField::new(name.into(), default_value, true);
    for field_type in field_types {
        input_field.push_type(field_type, &mut ctx.input_field_types);
    }
    input_field
}

/// Capitalizes first character.
/// Assumes 1-byte characters.
pub fn capitalize(s: &str) -> impl fmt::Display + '_ {
    struct Capitalized<'a>(&'a str);

    impl fmt::Display for Capitalized<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let first_char = if let Some(first_char) = self.0.chars().next() {
                first_char
            } else {
                return Ok(());
            };
            debug_assert!(first_char.is_ascii());
            let first_char = first_char.to_ascii_uppercase();

            fmt::Display::fmt(&first_char, f)?;
            f.write_str(&self.0[1..])
        }
    }

    Capitalized(s)
}

/// Appends an option of type T to a vector over T if the option is Some.
pub fn append_opt<T>(vec: &mut Vec<T>, opt: Option<T>) {
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
pub fn compound_id_field_name(pk: &PrimaryKey) -> String {
    pk.alias.clone().unwrap_or_else(|| {
        let pk_fields = pk.fields();
        let field_names: Vec<&str> = pk_fields.iter().map(|sf| sf.name()).collect();

        field_names.join("_")
    })
}
