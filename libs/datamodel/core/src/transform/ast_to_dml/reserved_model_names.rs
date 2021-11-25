use crate::{
    ast,
    diagnostics::{DatamodelError, Diagnostics},
};

pub fn is_reserved_type_name(name: &str) -> bool {
    RESERVED_NAMES.contains(&name)
}

pub(crate) fn validate_model_name(ast_model: &ast::Model, diagnostics: &mut Diagnostics) {
    if !is_reserved_type_name(&ast_model.name.name) {
        return;
    }

    diagnostics.push_error(DatamodelError::new_model_validation_error(
        &format!(
            "The model name `{}` is invalid. It is a reserved name. Please change it. Read more at https://pris.ly/d/naming-models",
            &ast_model.name.name
        ),
        &ast_model.name.name,
        ast_model.span,
    ))
}

pub(crate) fn validate_enum_name(ast_enum: &ast::Enum, diagnostics: &mut Diagnostics) {
    if !is_reserved_type_name(&ast_enum.name.name) {
        return;
    }

    diagnostics.push_error(DatamodelError::new_enum_validation_error(
        format!(
          "The enum name `{}` is invalid. It is a reserved name. Please change it. Read more at https://www.prisma.io/docs/reference/tools-and-interfaces/prisma-schema/data-model#naming-enums",
          &ast_enum.name.name
        ),
        ast_enum.name.name.to_owned(),
        ast_enum.span,
));
}

// The source of the following list is from prisma-client-js. Any edit should be done in both places.
// https://github.com/prisma/prisma/blob/master/src/packages/client/src/generation/generateClient.ts#L443
const RESERVED_NAMES: &[&str] = &[
    "PrismaClient",
    // JavaScript keywords
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];
