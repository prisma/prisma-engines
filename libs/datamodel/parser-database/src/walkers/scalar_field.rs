use crate::{
    ast,
    types::{DefaultAttribute, FieldWithArgs, ScalarField, ScalarType, SortOrder},
    walkers::{EnumWalker, ModelWalker, Walker},
    ParserDatabase, ScalarFieldType,
};
use diagnostics::Span;

/// A scalar field, as part of a model.
#[derive(Debug, Copy, Clone)]
pub struct ScalarFieldWalker<'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) field_id: ast::FieldId,
    pub(crate) db: &'db ParserDatabase,
    pub(crate) scalar_field: &'db ScalarField,
}

impl<'db> PartialEq for ScalarFieldWalker<'db> {
    fn eq(&self, other: &Self) -> bool {
        self.model_id == other.model_id && self.field_id == other.field_id
    }
}

impl<'db> Eq for ScalarFieldWalker<'db> {}

impl<'db> ScalarFieldWalker<'db> {
    /// The ID of the field node in the AST.
    pub fn field_id(self) -> ast::FieldId {
        self.field_id
    }

    /// The field node in the AST.
    pub fn ast_field(self) -> &'db ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    /// The name of the field.
    pub fn name(self) -> &'db str {
        self.ast_field().name()
    }

    /// The `@default()` AST attribute on the field, if any.
    pub fn default_attribute(self) -> Option<&'db ast::Attribute> {
        self.scalar_field
            .default
            .as_ref()
            .map(|d| d.default_attribute)
            .map(|id| &self.db.ast[id])
    }

    /// The final database name of the field. See crate docs for explanations on database names.
    pub fn database_name(self) -> &'db str {
        self.attributes()
            .mapped_name
            .map(|id| &self.db[id])
            .unwrap_or_else(|| self.name())
    }

    /// Does the field have an `@default(autoincrement())` attribute?
    pub fn is_autoincrement(self) -> bool {
        self.default_value().map(|dv| dv.is_autoincrement()).unwrap_or(false)
    }

    /// Does the field define a primary key by its own.
    pub fn is_single_pk(self) -> bool {
        self.model().field_is_single_pk(self.field_id)
    }

    /// Is there an `@ignore` attribute on the field?
    pub fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    /// Is the field optional / nullable?
    pub fn is_optional(self) -> bool {
        self.ast_field().arity.is_optional()
    }

    /// Is there an `@updateAt` attribute on the field?
    pub fn is_updated_at(self) -> bool {
        self.attributes().is_updated_at
    }

    fn attributes(self) -> &'db ScalarField {
        self.scalar_field
    }

    /// Is this field's type an enum? If yes, walk the enum.
    pub fn field_type_as_enum(self) -> Option<EnumWalker<'db>> {
        match self.scalar_field_type() {
            ScalarFieldType::Enum(enum_id) => Some(Walker {
                db: self.db,
                id: enum_id,
            }),
            _ => None,
        }
    }

    /// The name in the `@map(<name>)` attribute.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.attributes().mapped_name.map(|id| &self.db[id])
    }

    /// The model that contains the field.
    pub fn model(self) -> ModelWalker<'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub fn raw_native_type(self) -> Option<(&'db str, &'db str, &'db [String], Span)> {
        let db = self.db;
        self.attributes()
            .native_type
            .as_ref()
            .map(move |(datasource_name, name, args, span)| (&db[*datasource_name], &db[*name], args.as_slice(), *span))
    }

    /// Is the type of the field `Unsupported("...")`?
    pub fn is_unsupported(self) -> bool {
        matches!(self.ast_field().field_type, ast::FieldType::Unsupported(_, _))
    }

    /// The `@default()` attribute of the field, if any.
    pub fn default_value(self) -> Option<DefaultValueWalker<'db>> {
        self.attributes().default.as_ref().map(|d| DefaultValueWalker {
            model_id: self.model_id,
            field_id: self.field_id,
            db: self.db,
            default: d,
        })
    }

    /// The type of the field, with type aliases resolved.
    pub fn resolved_scalar_field_type(self) -> ScalarFieldType {
        match self.attributes().r#type {
            ScalarFieldType::Alias(id) => *self.db.alias_scalar_field_type(&id),
            other => other,
        }
    }

    /// The type of the field.
    pub fn scalar_field_type(self) -> ScalarFieldType {
        self.attributes().r#type
    }

    /// The type of the field in case it is a scalar type (not an enum, not a composite type).
    pub fn scalar_type(self) -> Option<ScalarType> {
        let mut tpe = &self.scalar_field.r#type;

        loop {
            match tpe {
                ScalarFieldType::BuiltInScalar(scalar) => return Some(*scalar),
                ScalarFieldType::Alias(alias_id) => tpe = &self.db.types.type_aliases[alias_id],
                _ => return None,
            }
        }
    }
}

/// An `@default()` attribute on a field.
#[derive(Clone, Copy)]
pub struct DefaultValueWalker<'db> {
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    db: &'db ParserDatabase,
    default: &'db DefaultAttribute,
}

impl<'db> DefaultValueWalker<'db> {
    /// The AST node of the attribute.
    pub fn ast_attribute(self) -> &'db ast::Attribute {
        &self.db.ast[self.default.default_attribute]
    }

    /// The value expression in the `@default` attribute.
    ///
    /// ```ignore
    /// score Int @default(0)
    ///                    ^
    /// ```
    pub fn value(self) -> &'db ast::Expression {
        &self.ast_attribute().arguments.arguments[self.default.argument_idx].value
    }

    /// Is this an `@default(autoincrement())`?
    pub fn is_autoincrement(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "autoincrement")
    }

    /// Is this an `@default(cuid())`?
    pub fn is_cuid(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "cuid")
    }

    /// Is this an `@default(dbgenerated())`?
    pub fn is_dbgenerated(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "dbgenerated")
    }

    /// Is this an `@default(auto())`?
    pub fn is_auto(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "auto")
    }

    /// Is this an `@default(now())`?
    pub fn is_now(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "now")
    }

    /// Is this an `@default(uuid())`?
    pub fn is_uuid(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "uuid")
    }

    /// The mapped name of the default value. Not applicable to all connectors. See crate docs for
    /// details on mapped names.
    ///
    /// ```ignore
    /// name String @default("george", map: "name_default_to_george")
    ///                                     ^^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    pub fn mapped_name(self) -> Option<&'db str> {
        self.default.mapped_name.map(|id| &self.db[id])
    }

    /// The field carrying the default attribute.
    ///
    /// ```ignore
    /// name String @default("george")
    /// ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    pub fn field(self) -> ScalarFieldWalker<'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id: self.field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, self.field_id)],
        }
    }
}

/// A scalar field as referenced in a key specification (id, index or unique).
#[derive(Copy, Clone)]
pub struct ScalarFieldAttributeWalker<'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) fields: &'db [FieldWithArgs],
    pub(crate) db: &'db ParserDatabase,
    pub(crate) field_arg_id: usize,
}

impl<'db> ScalarFieldAttributeWalker<'db> {
    fn args(self) -> &'db FieldWithArgs {
        &self.fields[self.field_arg_id]
    }

    /// The length argument on the field.
    ///
    /// ```ignore
    /// @@index(name(length: 10))
    ///                      ^^
    /// ```
    pub fn length(self) -> Option<u32> {
        self.args().length
    }

    /// The underlying scalar field.
    ///
    /// ```ignore
    /// model Test {
    ///   id          Int @id
    ///   name        String
    ///   ^^^^^^^^^^^^^^^^^^
    ///   kind        Int
    ///
    ///   @@index([name])
    /// }
    ///
    /// ```
    pub fn as_scalar_field(self) -> ScalarFieldWalker<'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id: self.args().field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, self.args().field_id)],
        }
    }

    /// The sort order (asc or desc) on the field.
    ///
    /// ```ignore
    /// @@index(name(sort: Desc))
    ///                    ^^^^
    /// ```
    pub fn sort_order(&self) -> Option<SortOrder> {
        self.args().sort_order
    }
}
