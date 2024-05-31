use super::Walker;
use crate::{ast, FileId, ScalarFieldType, ScalarType};
use diagnostics::Span;
use schema_ast::ast::{WithDocumentation, WithName};

/// A composite type, introduced with the `type` keyword in the schema.
///
/// Example:
///
/// ```prisma
/// type Address {
///     name String
///     streetName String
///     streetNumber Int
///     city String
///     zipCode Int
///     countryCode String
/// }
/// ```
pub type CompositeTypeWalker<'db> = Walker<'db, crate::CompositeTypeId>;

/// A field in a composite type.
pub type CompositeTypeFieldWalker<'db> = Walker<'db, (crate::CompositeTypeId, ast::FieldId)>;

impl<'db> CompositeTypeWalker<'db> {
    /// The ID of the composite type node in the AST.
    pub fn composite_type_id(self) -> (FileId, ast::CompositeTypeId) {
        self.id
    }

    /// The ID of the file containing the composite type.
    pub fn file_id(self) -> FileId {
        self.id.0
    }

    /// Is the composite type defined in a specific file?
    pub fn is_defined_in_file(self, file_id: FileId) -> bool {
        self.ast_composite_type().span.file_id == file_id
    }

    /// The composite type node in the AST.
    pub fn ast_composite_type(self) -> &'db ast::CompositeType {
        &self.db.asts[self.id]
    }

    /// The name of the composite type in the schema.
    pub fn name(self) -> &'db str {
        self.ast_composite_type().name()
    }

    /// Iterator over all the fields of the composite type.
    pub fn fields(self) -> impl ExactSizeIterator<Item = CompositeTypeFieldWalker<'db>> + Clone {
        self.ast_composite_type()
            .iter_fields()
            .map(move |(id, _)| self.walk((self.id, id)))
    }
}

impl<'db> CompositeTypeFieldWalker<'db> {
    fn field(self) -> &'db crate::types::CompositeTypeField {
        &self.db.types.composite_type_fields[&self.id]
    }

    /// The AST node for the field.
    pub fn ast_field(self) -> &'db ast::Field {
        &self.db.asts[self.id.0][self.id.1]
    }

    /// The composite type containing the field.
    pub fn composite_type(self) -> CompositeTypeWalker<'db> {
        self.db.walk(self.id.0)
    }

    /// The optional documentation string of the field.
    pub fn documentation(&self) -> Option<&str> {
        self.ast_field().documentation()
    }

    /// The name contained in the `@map()` attribute of the field, if any.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.field().mapped_name.map(|id| &self.db[id])
    }

    /// The ID of the field in the AST.
    pub fn field_id(self) -> ast::FieldId {
        self.id.1
    }

    /// The name of the field.
    pub fn name(self) -> &'db str {
        self.ast_field().name()
    }

    /// Is the field required, optional or a list?
    pub fn arity(self) -> ast::FieldArity {
        self.ast_field().arity
    }

    /// The type of the field, e.g. `String` in `streetName String?`.
    pub fn r#type(self) -> ScalarFieldType {
        self.field().r#type
    }

    /// The type of the field in case it is a scalar type (not an enum, not a composite type).
    pub fn scalar_type(self) -> Option<ScalarType> {
        match self.r#type() {
            ScalarFieldType::BuiltInScalar(scalar) => Some(scalar),
            _ => None,
        }
    }

    /// The `@default()` AST attribute on the field, if any.
    pub fn default_attribute(self) -> Option<&'db ast::Attribute> {
        self.field()
            .default
            .as_ref()
            .map(|d| &self.db.asts[(self.id.0 .0, d.default_attribute.1)])
    }

    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub fn raw_native_type(self) -> Option<(&'db str, &'db str, &'db [String], Span)> {
        let db = self.db;
        self.field()
            .native_type
            .as_ref()
            .map(move |(datasource_name, name, args, span)| (&db[*datasource_name], &db[*name], args.as_slice(), *span))
    }

    /// The value expression in the `@default` attribute.
    ///
    /// ```ignore
    /// score Int @default(0)
    ///                    ^
    /// ```
    pub fn default_value(self) -> Option<&'db ast::Expression> {
        let argument_idx = self.field().default.as_ref()?.argument_idx;
        let attr = self.default_attribute()?;
        Some(&attr.arguments.arguments[argument_idx].value)
    }

    /// The mapped name of the default value. Always `None` in composite types at the moment.
    ///
    /// ```ignore
    /// name String @default("george", map: "name_default_to_george")
    ///                                     ^^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    pub fn default_mapped_name(self) -> Option<&'db str> {
        self.field()
            .default
            .as_ref()
            .and_then(|d| d.mapped_name)
            .map(|id| &self.db[id])
    }

    /// The final database name of the field. See crate docs for explanations on database names.
    pub fn database_name(self) -> &'db str {
        self.field()
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| self.name())
    }
}
