use crate::{ast, types, ParserDatabase, ScalarFieldType, ScalarType};
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
#[derive(Copy, Clone)]
pub struct CompositeTypeWalker<'db> {
    pub(super) ctid: ast::CompositeTypeId,
    pub(super) db: &'db ParserDatabase,
}

impl<'db> PartialEq for CompositeTypeWalker<'db> {
    fn eq(&self, other: &Self) -> bool {
        self.ctid == other.ctid
    }
}

impl<'db> CompositeTypeWalker<'db> {
    /// The ID of the composite type node in the AST.
    pub fn composite_type_id(self) -> ast::CompositeTypeId {
        self.ctid
    }

    /// The composite type node in the AST.
    pub fn ast_composite_type(self) -> &'db ast::CompositeType {
        &self.db.ast()[self.ctid]
    }

    /// The name of the composite type in the schema.
    pub fn name(self) -> &'db str {
        self.db.ast[self.ctid].name()
    }

    /// Get the field with the given ID.
    pub fn field(self, field_id: ast::FieldId) -> CompositeTypeFieldWalker<'db> {
        let field = &self.db.types.composite_type_fields[&(self.ctid, field_id)];

        CompositeTypeFieldWalker {
            ctid: self.ctid,
            field_id,
            field,
            db: self.db,
        }
    }

    /// Iterator over all the fields of the composite type.
    pub fn fields(self) -> impl Iterator<Item = CompositeTypeFieldWalker<'db>> {
        let db = self.db;
        db.types
            .composite_type_fields
            .range((self.ctid, ast::FieldId::MIN)..(self.ctid, ast::FieldId::MAX))
            .map(move |((ctid, field_id), field)| CompositeTypeFieldWalker {
                ctid: *ctid,
                field_id: *field_id,
                field,
                db,
            })
    }
}

/// A field in a composite type.
#[derive(Clone, Copy)]
pub struct CompositeTypeFieldWalker<'db> {
    pub(super) ctid: ast::CompositeTypeId,
    pub(super) field_id: ast::FieldId,
    pub(super) field: &'db types::CompositeTypeField,
    pub(super) db: &'db ParserDatabase,
}

impl<'db> PartialEq for CompositeTypeFieldWalker<'db> {
    fn eq(&self, other: &Self) -> bool {
        self.ctid == other.ctid && self.field_id == other.field_id
    }
}

impl<'db> CompositeTypeFieldWalker<'db> {
    /// The AST node for the field.
    pub fn ast_field(self) -> &'db ast::Field {
        &self.db.ast[self.ctid][self.field_id]
    }

    /// The composite type containing the field.
    pub fn composite_type(self) -> CompositeTypeWalker<'db> {
        CompositeTypeWalker {
            ctid: self.ctid,
            db: self.db,
        }
    }

    /// The optional documentation string of the field.
    pub fn documentation(&self) -> Option<&str> {
        self.ast_field().documentation()
    }

    /// The name contained in the `@map()` attribute of the field, if any.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.field.mapped_name.map(|id| &self.db[id])
    }

    /// The ID of the field in the AST.
    pub fn field_id(self) -> ast::FieldId {
        self.field_id
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
    pub fn r#type(self) -> &'db ScalarFieldType {
        &self.field.r#type
    }

    /// The type of the field in case it is a scalar type (not an enum, not a composite type).
    pub fn scalar_type(self) -> Option<ScalarType> {
        match self.r#type() {
            ScalarFieldType::BuiltInScalar(scalar) => Some(*scalar),
            _ => None,
        }
    }

    /// The `@default()` AST attribute on the field, if any.
    pub fn default_attribute(self) -> Option<&'db ast::Attribute> {
        self.field.default.as_ref().map(|d| &self.db.ast[d.default_attribute])
    }

    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub fn raw_native_type(self) -> Option<(&'db str, &'db str, &'db [String], Span)> {
        let db = self.db;
        self.field
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
        let argument_idx = self.field.default.as_ref()?.argument_idx;
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
        self.field
            .default
            .as_ref()
            .and_then(|d| d.mapped_name)
            .map(|id| &self.db[id])
    }

    /// Get the database name of the composite field.
    pub fn get_field_database_name(self, field_id: ast::FieldId) -> &'db str {
        self.db.types.composite_type_fields[&(self.ctid, field_id)]
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| self.db.ast[self.ctid][field_id].name())
    }

    /// The final database name of the field. See crate docs for explanations on database names.
    pub(crate) fn database_name(self) -> &'db str {
        self.field
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| self.name())
    }
}
