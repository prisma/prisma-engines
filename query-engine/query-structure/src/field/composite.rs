use crate::{parent_container::ParentContainer, CompositeType};
use psl::{
    parser_database::{self as db, ScalarFieldId},
    schema_ast::ast::{self, FieldArity},
};
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompositeFieldId {
    InModel(ScalarFieldId),
    InCompositeType((db::CompositeTypeId, ast::FieldId)),
}

pub type CompositeField = crate::Zipper<CompositeFieldId>;
pub type CompositeFieldRef = CompositeField;

impl CompositeField {
    pub fn borrowed_name<'a>(&self, schema: &'a psl::ValidatedSchema) -> &'a str {
        match self.id {
            CompositeFieldId::InModel(sfid) => schema.db.walk(sfid).name(),
            CompositeFieldId::InCompositeType(id) => schema.db.walk(id).name(),
        }
    }

    pub fn arity(&self) -> FieldArity {
        match self.id {
            CompositeFieldId::InModel(sfid) => self.dm.walk(sfid).ast_field().arity,
            CompositeFieldId::InCompositeType(id) => self.dm.walk(id).arity(),
        }
    }

    pub fn typ(&self) -> CompositeType {
        let id = match self.id {
            CompositeFieldId::InModel(sfid) => self.dm.walk(sfid).scalar_field_type().as_composite_type().unwrap(),
            CompositeFieldId::InCompositeType(ctid) => self.dm.walk(ctid).r#type().as_composite_type().unwrap(),
        };
        self.dm.find_composite_type_by_id(id)
    }

    pub fn is_list(&self) -> bool {
        matches!(self.arity(), FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity(), FieldArity::Required)
    }

    pub fn is_optional(&self) -> bool {
        matches!(self.arity(), FieldArity::Optional)
    }

    pub fn name(&self) -> &str {
        match self.id {
            CompositeFieldId::InModel(sfid) => self.dm.walk(sfid).name(),
            CompositeFieldId::InCompositeType(id) => self.dm.walk(id).name(),
        }
    }

    pub fn db_name(&self) -> &str {
        match self.id {
            CompositeFieldId::InModel(sfid) => self.dm.walk(sfid).database_name(),
            CompositeFieldId::InCompositeType(id) => self.dm.walk(id).database_name(),
        }
    }

    pub fn container(&self) -> ParentContainer {
        match self.id {
            CompositeFieldId::InModel(id) => {
                let id = self.dm.walk(id).model().id;
                self.dm.find_model_by_id(id).into()
            }
            CompositeFieldId::InCompositeType((ct_id, _)) => self.dm.find_composite_type_by_id(ct_id).into(),
        }
    }
}

impl Display for CompositeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.container().name(), self.name())
    }
}

impl std::fmt::Debug for CompositeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CompositeField")
            .field(&format!("{}.{}", self.container().name(), self.name()))
            .finish()
    }
}
