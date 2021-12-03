use super::ModelWalker;
use crate::{
    ast,
    types::{FieldWithArgs, ScalarField},
    ParserDatabase, ScalarFieldType,
};
use diagnostics::Span;
use dml::{
    default_value::{DefaultKind, DefaultValue},
    model::SortOrder,
};

#[derive(Copy, Clone)]
pub struct ScalarFieldWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) field_id: ast::FieldId,
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) scalar_field: &'db ScalarField<'ast>,
}

impl<'ast, 'db> PartialEq for ScalarFieldWalker<'ast, 'db> {
    fn eq(&self, other: &Self) -> bool {
        self.model_id == other.model_id && self.field_id == other.field_id
    }
}

impl<'ast, 'db> Eq for ScalarFieldWalker<'ast, 'db> {}

impl<'ast, 'db> ScalarFieldWalker<'ast, 'db> {
    pub fn field_id(self) -> ast::FieldId {
        self.field_id
    }

    pub fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub fn name(self) -> &'ast str {
        self.ast_field().name()
    }

    pub fn default_attribute(self) -> Option<&'ast ast::Attribute> {
        self.scalar_field.default_attribute
    }

    pub fn final_database_name(self) -> &'ast str {
        self.attributes().mapped_name.unwrap_or_else(|| self.name())
    }

    pub fn is_autoincrement(self) -> bool {
        matches!(&self.scalar_field.default.as_ref().map(|d| d.kind()), Some(DefaultKind::Expression(expr)) if expr.is_autoincrement())
    }

    pub fn is_ignored(self) -> bool {
        self.attributes().is_ignored
    }

    pub fn is_optional(self) -> bool {
        self.ast_field().arity.is_optional()
    }

    pub fn is_updated_at(self) -> bool {
        self.attributes().is_updated_at
    }

    pub(crate) fn attributes(self) -> &'db ScalarField<'ast> {
        self.scalar_field
    }

    /// The name in the `@map(<name>)` attribute.
    pub fn mapped_name(self) -> Option<&'ast str> {
        self.attributes().mapped_name
    }

    pub fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    /// (attribute scope, native type name, arguments, span)
    ///
    /// For example: `@db.Text` would translate to ("db", "Text", &[], <the span>)
    pub fn raw_native_type(self) -> Option<(&'ast str, &'ast str, &'db [String], Span)> {
        self.attributes()
            .native_type
            .as_ref()
            .map(move |(datasource_name, name, args, span)| (*datasource_name, *name, args.as_slice(), *span))
    }

    pub fn is_unsupported(self) -> bool {
        matches!(self.ast_field().field_type, ast::FieldType::Unsupported(_, _))
    }

    pub fn default_value(self) -> Option<DefaultValueWalker<'ast, 'db>> {
        self.attributes().default.as_ref().map(|d| DefaultValueWalker {
            model_id: self.model_id,
            field_id: self.field_id,
            db: self.db,
            default: d,
        })
    }

    pub fn scalar_field_type(self) -> ScalarFieldType {
        self.attributes().r#type
    }

    pub fn scalar_type(self) -> Option<dml::scalars::ScalarType> {
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

#[derive(Clone, Copy)]
pub struct DefaultValueWalker<'ast, 'db> {
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    db: &'db ParserDatabase<'ast>,
    default: &'db DefaultValue,
}

impl<'ast, 'db> DefaultValueWalker<'ast, 'db> {
    pub fn default(self) -> &'db DefaultValue {
        self.default
    }

    pub fn mapped_name(self) -> Option<&'db str> {
        self.default.db_name()
    }

    pub fn field(self) -> ScalarFieldWalker<'ast, 'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id: self.field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, self.field_id)],
        }
    }
}

#[derive(Copy, Clone)]
pub struct ScalarFieldAttributeWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) fields: &'db [FieldWithArgs],
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) field_arg_id: usize,
}

impl<'ast, 'db> ScalarFieldAttributeWalker<'ast, 'db> {
    fn args(self) -> &'db FieldWithArgs {
        &self.fields[self.field_arg_id]
    }

    pub fn length(self) -> Option<u32> {
        self.args().length
    }

    pub fn as_scalar_field(self) -> ScalarFieldWalker<'ast, 'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id: self.args().field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, self.args().field_id)],
        }
    }

    pub fn sort_order(&self) -> Option<SortOrder> {
        self.args().sort_order
    }
}
