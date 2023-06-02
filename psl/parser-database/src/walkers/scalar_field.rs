use crate::{
    ast::{self, WithName},
    types::{DefaultAttribute, FieldWithArgs, OperatorClassStore, ScalarField, ScalarType, SortOrder},
    walkers::*,
    OperatorClass, ParserDatabase, ScalarFieldId, ScalarFieldType,
};
use diagnostics::Span;
use either::Either;

/// A scalar field, as part of a model.
pub type ScalarFieldWalker<'db> = Walker<'db, ScalarFieldId>;

impl<'db> ScalarFieldWalker<'db> {
    /// The ID of the field node in the AST.
    pub fn field_id(self) -> ast::FieldId {
        self.attributes().field_id
    }

    /// The field node in the AST.
    pub fn ast_field(self) -> &'db ast::Field {
        let ScalarField { model_id, field_id, .. } = self.attributes();
        &self.db.ast[*model_id][*field_id]
    }

    /// Is this field unique? This method will return true if:
    ///
    /// - The field has an `@id` or `@unique` attribute.
    /// - There is an `@@id` or `@@unique` on the model that contains __only__ this field.
    pub fn is_unique(self) -> bool {
        let model = self.model();

        if let Some(true) = model
            .primary_key()
            .map(|pk| pk.contains_exactly_fields_by_id(&[self.id]))
        {
            return true;
        }

        self.model().indexes().any(|idx| {
            let mut fields = idx.fields();
            idx.is_unique() && fields.len() == 1 && fields.next().map(|f| f.field_id()) == Some(self.field_id())
        })
    }

    /// The name of the field.
    pub fn name(self) -> &'db str {
        self.ast_field().name()
    }

    /// The `@default()` AST attribute on the field, if any.
    pub fn default_attribute(self) -> Option<&'db ast::Attribute> {
        self.attributes()
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
        self.model().field_is_single_pk(self.field_id())
    }

    /// Is the field part of a compound primary key.
    pub fn is_part_of_a_compound_pk(self) -> bool {
        self.model().field_is_part_of_a_compound_pk(self.field_id())
    }

    /// Is there an `@ignore` attribute on the field?
    pub fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    /// Is the field optional / nullable?
    pub fn is_optional(self) -> bool {
        self.ast_field().arity.is_optional()
    }

    /// Is the field a list
    pub fn is_list(self) -> bool {
        self.ast_field().arity.is_list()
    }

    /// Is there an `@updatedAt` attribute on the field?
    pub fn is_updated_at(self) -> bool {
        self.attributes().is_updated_at
    }

    fn attributes(self) -> &'db ScalarField {
        &self.db.types[self.id]
    }

    /// Is this field's type an enum? If yes, walk the enum.
    pub fn field_type_as_enum(self) -> Option<EnumWalker<'db>> {
        self.scalar_field_type().as_enum().map(|id| self.db.walk(id))
    }

    /// The name in the `@map(<name>)` attribute.
    pub fn mapped_name(self) -> Option<&'db str> {
        self.attributes().mapped_name.map(|id| &self.db[id])
    }

    /// The model that contains the field.
    pub fn model(self) -> ModelWalker<'db> {
        self.walk(self.attributes().model_id)
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
        let ScalarField { default, .. } = self.attributes();
        default.as_ref().map(|default| DefaultValueWalker {
            field_id: self.id,
            db: self.db,
            default,
        })
    }

    /// The type of the field.
    pub fn scalar_field_type(self) -> ScalarFieldType {
        self.attributes().r#type
    }

    /// The type of the field in case it is a scalar type (not an enum, not a composite type).
    pub fn scalar_type(self) -> Option<ScalarType> {
        match self.attributes().r#type {
            ScalarFieldType::BuiltInScalar(scalar) => Some(scalar),
            _ => None,
        }
    }
}

/// An `@default()` attribute on a field.
#[derive(Clone, Copy)]
pub struct DefaultValueWalker<'db> {
    pub(super) field_id: ScalarFieldId,
    pub(super) db: &'db ParserDatabase,
    pub(super) default: &'db DefaultAttribute,
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

    /// Is this an `@default(nanoid())`?
    pub fn is_nanoid(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "nanoid")
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

    /// Is this an `@default(sequence())`?
    pub fn is_sequence(self) -> bool {
        matches!(self.value(), ast::Expression::Function(name, _, _) if name == "sequence")
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
        self.db.walk(self.field_id)
    }
}

/// An operator class defines the operators allowed in an index. Mostly
/// a PostgreSQL thing.
#[derive(Copy, Clone)]
pub struct OperatorClassWalker<'db> {
    pub(crate) class: &'db OperatorClassStore,
    pub(crate) db: &'db ParserDatabase,
}

impl<'db> OperatorClassWalker<'db> {
    /// Gets the operator class of the indexed field.
    ///
    /// ```ignore
    /// @@index(name(ops: InetOps))
    /// //                ^ Either::Left(InetOps)
    /// @@index(name(ops: raw("tsvector_ops")))
    /// //                ^ Either::Right("tsvector_ops")
    pub fn get(self) -> Either<OperatorClass, &'db str> {
        match self.class.inner {
            Either::Left(class) => Either::Left(class),
            Either::Right(id) => Either::Right(&self.db[id]),
        }
    }
}

/// A scalar field as referenced in a key specification (id, index or unique).
#[derive(Copy, Clone)]
pub struct ScalarFieldAttributeWalker<'db> {
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

    /// A custom operator class to control the operators catched by the index.
    ///
    /// ```ignore
    /// @@index([name(ops: InetOps)], type: Gist)
    ///                    ^^^^^^^
    /// ```
    pub fn operator_class(self) -> Option<OperatorClassWalker<'db>> {
        self.args()
            .operator_class
            .as_ref()
            .map(|class| OperatorClassWalker { class, db: self.db })
    }

    /// The underlying field.
    ///
    /// ```ignore
    /// // either this
    /// model Test {
    ///   id          Int @id
    ///   name        String
    ///   ^^^^^^^^^^^^^^^^^^
    ///   kind        Int
    ///
    ///   @@index([name])
    /// }
    ///
    /// // or this
    /// type A {
    ///   field String
    ///   ^^^^^^^^^^^^
    /// }
    ///
    /// model Test {
    ///   id Int @id
    ///   a  A
    ///
    ///   @@index([a.field])
    /// }
    /// ```
    pub fn as_index_field(self) -> IndexFieldWalker<'db> {
        match self.args().path.field_in_index() {
            Either::Left(id) => IndexFieldWalker::new(self.db.walk(id)),
            Either::Right(ctid) => IndexFieldWalker::new(self.db.walk(ctid)),
        }
    }

    /// Gives the full path from the current model to the field included in the index.
    /// For example, if the field is through two composite types:
    ///
    /// ```ignore
    /// type A {
    ///   field Int
    /// }
    ///
    /// type B {
    ///   a A
    /// }
    ///
    /// model C {
    ///   id Int @id
    ///   b  B
    ///
    ///   @@index([b.a.field])
    /// }
    /// ```
    ///
    /// The method would return a vector from model to the final field:
    ///
    /// ```ignore
    /// vec![("b", None), ("a", Some("B")), ("field", Some("A"))];
    /// ```
    ///
    /// The first part of the tuple is the name of the field, the second part is
    /// the name of the composite type.
    ///
    /// This method prefers the prisma side naming, and should not be used when
    /// writing to the database.
    pub fn as_path_to_indexed_field(self) -> Vec<(&'db str, Option<&'db str>)> {
        let path = &self.args().path;
        let root_name = self.db.walk(path.root()).name();
        let mut result = vec![(root_name, None)];

        for (ctid, field_id) in path.path() {
            let ct = &self.db.ast[*ctid];
            let field = ct[*field_id].name();

            result.push((field, Some(ct.name())));
        }

        result
    }

    /// Similar to the method [`as_path_to_indexed_field`], but prefers the
    /// mapped names and is to be used when defining indices in the database.
    ///
    /// [`as_path_to_indexed_field`]: struct.ScalarFieldAttributeWalker
    pub fn as_mapped_path_to_indexed_field(self) -> Vec<(&'db str, Option<&'db str>)> {
        let path = &self.args().path;
        let root = {
            let mapped = &self.db.types[path.root()].mapped_name;

            mapped
                .and_then(|id| self.db.interner.get(id))
                .unwrap_or_else(|| self.db.walk(path.root()).name())
        };

        let mut result = vec![(root, None)];

        for (ctid, field_id) in path.path() {
            let ct = &self.db.ast[*ctid];

            let field = &self.db.types.composite_type_fields[&(*ctid, *field_id)]
                .mapped_name
                .and_then(|id| self.db.interner.get(id))
                .unwrap_or_else(|| ct[*field_id].name());

            result.push((field, Some(ct.name())));
        }

        result
    }

    /// The sort order (asc or desc) on the field.
    ///
    /// ```ignore
    /// @@index(name(sort: Desc))
    ///                    ^^^^
    /// ```
    pub fn sort_order(self) -> Option<SortOrder> {
        self.args().sort_order
    }
}
