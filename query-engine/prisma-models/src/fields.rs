use crate::pk::PrimaryKey;
use crate::*;
use psl::parser_database::ScalarFieldType;
use std::collections::BTreeSet;

pub type Fields = Model;

impl Fields {
    pub fn id(&self) -> Option<PrimaryKey> {
        self.walker().primary_key().map(|pk| PrimaryKey {
            fields: pk
                .fields()
                .map(|f| self.dm.clone().zip(ScalarFieldId::InModel(f.id)))
                .collect::<Vec<_>>(),
            alias: pk.name().map(ToOwned::to_owned),
        })
    }

    pub fn compound_id(&self) -> Option<PrimaryKey> {
        self.id().filter(|pk| pk.fields().len() > 1)
    }

    pub fn updated_at_fields(&self) -> impl Iterator<Item = ScalarFieldRef> {
        self.scalar_fields().into_iter().filter(|sf| sf.is_updated_at())
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        let model = self;
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .scalar_fields()
            .filter(|sf| {
                !matches!(
                    sf.scalar_field_type(),
                    ScalarFieldType::CompositeType(_) | ScalarFieldType::Unsupported(_)
                )
            })
            .map(|rf| internal_data_model.clone().zip(ScalarFieldId::InModel(rf.id)))
            .collect()
    }

    pub fn relation_fields(&self) -> Vec<RelationFieldRef> {
        let model = self;
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .relation_fields()
            .filter(|rf| !rf.relation().is_ignored())
            .map(|rf| internal_data_model.clone().zip(rf.id))
            .collect()
    }

    pub fn composite_fields(&self) -> Vec<CompositeFieldRef> {
        let model = self;
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .scalar_fields()
            .filter(|sf| sf.scalar_field_type().as_composite_type().is_some())
            .map(|sf| internal_data_model.clone().zip(CompositeFieldId::InModel(sf.id)))
            .collect()
    }

    pub fn non_relational_fields(&self) -> Vec<Field> {
        self.scalar_fields()
            .into_iter()
            .map(Field::from)
            .chain(self.composite_fields().into_iter().map(Field::from))
            .collect()
    }

    pub fn find_many_from_scalar(&self, names: &BTreeSet<String>) -> Vec<ScalarFieldRef> {
        self.scalar_fields()
            .into_iter()
            .filter(|field| names.contains(field.name()))
            .collect()
    }

    pub fn find_from_all(&self, prisma_name: &str) -> crate::Result<Field> {
        let model = self;
        let internal_data_model = model.internal_data_model();
        let model_walker = internal_data_model.walk(model.id);
        let mut scalar_fields = model_walker.scalar_fields();
        let mut relation_fields = model_walker.relation_fields();
        scalar_fields
            .find(|f| f.name() == prisma_name)
            .map(|w| Field::from((internal_data_model.clone(), w)))
            .or_else(|| {
                relation_fields
                    .find(|f| f.name() == prisma_name)
                    .map(|w| Field::from((internal_data_model.clone(), w)))
            })
            .ok_or_else(|| DomainError::FieldNotFound {
                name: prisma_name.to_string(),
                container_name: self.name().to_owned(),
                container_type: "model",
            })
    }

    /// Non-virtual: Fields actually existing on the database level, this (currently) excludes relations, which are
    /// purely virtual on a model.
    pub fn find_from_non_virtual_by_db_name(&self, db_name: &str) -> crate::Result<Field> {
        self.fields()
            .find(|f| f.db_name() == db_name)
            .ok_or_else(|| DomainError::FieldNotFound {
                name: db_name.to_string(),
                container_name: self.name().to_owned(),
                container_type: "model",
            })
    }

    pub fn find_from_scalar(&self, name: &str) -> crate::Result<ScalarFieldRef> {
        self.scalar_fields()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::ScalarFieldNotFound {
                name: name.to_string(),
                container_name: self.name().to_owned(),
                container_type: "model",
            })
    }

    pub fn find_from_relation_fields(&self, name: &str) -> Result<RelationFieldRef> {
        self.relation_fields()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::RelationFieldNotFound {
                name: name.to_string(),
                model: self.name().to_owned(),
            })
    }

    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        self.walker()
            .fields()
            .map(|w| Field::from((self.dm.clone(), w)))
            .filter(|f| !f.is_ignored() && !f.is_unsupported())
    }
}
