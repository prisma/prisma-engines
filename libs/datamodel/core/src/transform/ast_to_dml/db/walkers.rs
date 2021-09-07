use super::{
    types::{ModelData, RelationField},
    ParserDatabase,
};
use crate::{ast, common::constraint_names::ConstraintNames};
use std::borrow::Cow;

impl<'ast> ParserDatabase<'ast> {
    pub(crate) fn walk_model(&self, model_id: ast::ModelId) -> ModelWalker<'ast, '_> {
        ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.models[&model_id],
        }
    }

    pub(crate) fn walk_models(&self) -> impl Iterator<Item = ModelWalker<'ast, '_>> + '_ {
        self.ast.iter_models().map(move |(model_id, _)| ModelWalker {
            model_id,
            db: self,
            model_attributes: &self.types.models[&model_id],
        })
    }
}

pub(crate) struct ModelWalker<'ast, 'db> {
    pub(super) model_id: ast::ModelId,
    pub(super) db: &'db ParserDatabase<'ast>,
    pub(super) model_attributes: &'db ModelData<'ast>,
}

impl<'ast, 'db> ModelWalker<'ast, 'db> {
    pub(crate) fn attributes(&self) -> &'db ModelData<'ast> {
        self.model_attributes
    }

    fn final_database_name(&self) -> &'ast str {
        self.model_attributes
            .mapped_name
            .unwrap_or_else(|| &self.db.ast[self.model_id].name.name)
    }

    fn get_field_db_names<'a>(&'a self, fields: &'a [ast::FieldId]) -> impl Iterator<Item = &'ast str> + 'a {
        fields.into_iter().map(move |&field_id| {
            self.db.types.scalar_fields[&(self.model_id, field_id)]
                .mapped_name
                .unwrap_or_else(|| &self.db.ast[self.model_id][field_id].name.name)
        })
    }

    pub(crate) fn walk_indexes(&self) -> impl Iterator<Item = IndexWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.model_attributes
            .indexes
            .iter()
            .map(move |(index, index_attribute)| IndexWalker {
                model_id,
                index,
                db,
                index_attribute,
            })
    }

    pub(crate) fn walk_relation_fields(&self) -> impl Iterator<Item = RelationFieldWalker<'ast, 'db>> + 'db {
        let model_id = self.model_id;
        let db = self.db;

        self.db
            .iter_model_relation_fields(self.model_id)
            .map(move |(field_id, relation_field)| RelationFieldWalker {
                model_id,
                field_id,
                db,
                relation_field,
            })
    }
}

pub(crate) struct IndexWalker<'ast, 'db> {
    model_id: ast::ModelId,
    index: &'ast ast::Attribute,
    db: &'db ParserDatabase<'ast>,
    index_attribute: &'db super::types::IndexData<'ast>,
}

impl<'ast, 'db> IndexWalker<'ast, 'db> {
    pub(crate) fn ast_attribute(&self) -> &'ast ast::Attribute {
        self.index
    }

    pub(crate) fn final_database_name(&self) -> Cow<'ast, str> {
        if let Some(mapped_name) = &self.index_attribute.db_name {
            return Cow::Borrowed(mapped_name);
        }

        let model = self.db.walk_model(self.model_id);
        let model_db_name = model.final_database_name();
        let field_db_names: Vec<&str> = model.get_field_db_names(&self.index_attribute.fields).collect();

        ConstraintNames::index_name(
            model_db_name,
            &field_db_names,
            self.index_attribute.is_unique,
            self.db.active_connector(),
        )
        .into()
    }

    pub(crate) fn attribute(&self) -> &'db super::types::IndexData<'ast> {
        self.index_attribute
    }
}

pub(crate) struct RelationFieldWalker<'ast, 'db> {
    model_id: ast::ModelId,
    field_id: ast::FieldId,
    db: &'db ParserDatabase<'ast>,
    relation_field: &'db RelationField<'ast>,
}

impl<'ast, 'db> RelationFieldWalker<'ast, 'db> {
    pub(crate) fn field_id(&self) -> ast::FieldId {
        self.field_id
    }

    pub(crate) fn ast_field(&self) -> &'ast ast::Field {
        &self.db.ast[self.model_id][self.field_id]
    }

    pub(crate) fn attributes(&self) -> &'db RelationField<'ast> {
        self.relation_field
    }

    /// This will be None for virtual relation fields (when no `fields` argument is passed).
    pub(crate) fn final_foreign_key_name(&self) -> Option<Cow<'ast, str>> {
        self.attributes().fk_name.map(Cow::Borrowed).or_else(|| {
            let fields = self.relation_field.fields.as_ref()?;
            let model = self.db.walk_model(self.model_id);
            let table_name = model.final_database_name();
            let column_names: Vec<&str> = model.get_field_db_names(&fields).collect();

            Some(
                ConstraintNames::foreign_key_constraint_name(table_name, &column_names, self.db.active_connector())
                    .into(),
            )
        })
    }
}
