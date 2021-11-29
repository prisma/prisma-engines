use std::borrow::Cow;

use dml::{default_value::DefaultValue, model::SortOrder, native_type_instance::NativeTypeInstance};

use super::ModelWalker;
use crate::{
    ast,
    common::constraint_names::ConstraintNames,
    transform::ast_to_dml::db::{types::FieldWithArgs, ParserDatabase, ScalarField, ScalarFieldType},
};

#[derive(Copy, Clone)]
pub(crate) struct ScalarFieldWalker<'ast, 'db> {
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
    #[allow(dead_code)] // we'll need this
    pub(crate) fn field_id(self) -> ast::FieldId {
        self.field_id
    }

    pub(crate) fn ast_field(self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn name(self) -> &'ast str {
        self.ast_field().name()
    }

    pub(crate) fn final_database_name(self) -> &'ast str {
        self.attributes().mapped_name.unwrap_or_else(|| self.name())
    }

    pub(crate) fn is_optional(self) -> bool {
        self.ast_field().arity.is_optional()
    }

    pub(crate) fn attributes(self) -> &'db ScalarField<'ast> {
        self.scalar_field
    }

    pub(crate) fn model(self) -> ModelWalker<'ast, 'db> {
        ModelWalker {
            model_id: self.model_id,
            db: self.db,
            model_attributes: &self.db.types.model_attributes[&self.model_id],
        }
    }

    pub(crate) fn native_type_instance(self) -> Option<NativeTypeInstance> {
        self.scalar_field.native_type.as_ref().map(|(name, args)| {
            self.db
                .active_connector()
                .parse_native_type(name, args.clone())
                .unwrap()
        })
    }

    pub(crate) fn is_unsupported(self) -> bool {
        matches!(self.ast_field().field_type, ast::FieldType::Unsupported(_, _))
    }

    pub(crate) fn default_value(self) -> Option<DefaultValueWalker<'ast, 'db>> {
        self.attributes().default.as_ref().map(|d| DefaultValueWalker {
            model_id: self.model_id,
            field_id: self.field_id,
            db: self.db,
            default: d,
        })
    }

    pub(crate) fn scalar_type(self) -> Option<dml::scalars::ScalarType> {
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
pub(crate) struct DefaultValueWalker<'ast, 'db> {
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    db: &'db ParserDatabase<'ast>,
    default: &'db DefaultValue,
}

impl<'ast, 'db> DefaultValueWalker<'ast, 'db> {
    pub(crate) fn constraint_name(self) -> Cow<'db, str> {
        self.default.db_name().map(Cow::from).unwrap_or_else(|| {
            let name = ConstraintNames::default_name(
                self.field().model().final_database_name(),
                self.field().final_database_name(),
                self.db.active_connector(),
            );

            Cow::from(name)
        })
    }

    pub(crate) fn default(self) -> &'db DefaultValue {
        self.default
    }

    fn field(self) -> ScalarFieldWalker<'ast, 'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id: self.field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, self.field_id)],
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ScalarFieldAttributeWalker<'ast, 'db> {
    pub(crate) model_id: ast::ModelId,
    pub(crate) fields: &'db [FieldWithArgs],
    pub(crate) db: &'db ParserDatabase<'ast>,
    pub(crate) field_arg_id: usize,
}

impl<'ast, 'db> ScalarFieldAttributeWalker<'ast, 'db> {
    fn args(self) -> &'db FieldWithArgs {
        &self.fields[self.field_arg_id]
    }

    pub(crate) fn length(self) -> Option<u32> {
        self.args().length
    }

    pub(crate) fn as_scalar_field(self) -> ScalarFieldWalker<'ast, 'db> {
        ScalarFieldWalker {
            model_id: self.model_id,
            field_id: self.args().field_id,
            db: self.db,
            scalar_field: &self.db.types.scalar_fields[&(self.model_id, self.args().field_id)],
        }
    }

    pub(crate) fn sort_order(&self) -> Option<SortOrder> {
        self.args().sort_order
    }
}
