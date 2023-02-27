use crate::pk::PrimaryKey;
use crate::*;
use psl::parser_database::ScalarFieldType;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct Fields {
    model: Model,
}

impl Fields {
    pub(crate) fn new(model: Model) -> Fields {
        Fields { model }
    }

    pub fn id(&self) -> Option<PrimaryKey> {
        self.model.walker().primary_key().map(|pk| PrimaryKey {
            fields: pk
                .fields()
                .map(|f| self.model.dm.clone().zip(ScalarFieldId::InModel(f.id)))
                .collect::<Vec<_>>(),
            alias: pk.name().map(ToOwned::to_owned),
        })
    }

    pub fn compound_id(&self) -> Option<PrimaryKey> {
        self.id().filter(|pk| pk.fields().len() > 1)
    }

    pub fn updated_at(&self) -> impl Iterator<Item = ScalarFieldRef> {
        self.scalar().into_iter().filter(|sf| sf.is_updated_at())
    }

    pub fn scalar(&self) -> Vec<ScalarFieldRef> {
        let model = self.model();
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

    pub fn relation(&self) -> Vec<RelationFieldRef> {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .relation_fields()
            .filter(|rf| !rf.relation().is_ignored())
            .map(|rf| internal_data_model.clone().zip(rf.id))
            .collect()
    }

    pub fn composite(&self) -> Vec<CompositeFieldRef> {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        internal_data_model
            .walk(model.id)
            .scalar_fields()
            .filter(|sf| sf.scalar_field_type().as_composite_type().is_some())
            .map(|sf| internal_data_model.clone().zip(CompositeFieldId::InModel(sf.id)))
            .collect()
    }

    pub fn non_relational(&self) -> Vec<Field> {
        self.scalar()
            .into_iter()
            .map(Field::from)
            .chain(self.composite().into_iter().map(Field::from))
            .collect()
    }

    pub fn find_many_from_scalar(&self, names: &BTreeSet<String>) -> Vec<ScalarFieldRef> {
        self.scalar()
            .into_iter()
            .filter(|field| names.contains(field.name()))
            .collect()
    }

    pub fn find_from_all(&self, prisma_name: &str) -> crate::Result<Field> {
        let model = self.model();
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
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    /// Non-virtual: Fields actually existing on the database level, this (currently) excludes relations, which are
    /// purely virtual on a model.
    pub fn find_from_non_virtual_by_db_name(&self, db_name: &str) -> crate::Result<Field> {
        self.filter_all(|f| f.db_name() == db_name)
            .into_iter()
            .next()
            .ok_or_else(|| DomainError::FieldNotFound {
                name: db_name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    pub fn find_from_scalar(&self, name: &str) -> crate::Result<ScalarFieldRef> {
        self.scalar()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::ScalarFieldNotFound {
                name: name.to_string(),
                container_name: self.model().name().to_owned(),
                container_type: "model",
            })
    }

    fn model(&self) -> &ModelRef {
        &self.model
    }

    pub fn find_from_relation_fields(&self, name: &str) -> Result<RelationFieldRef> {
        self.relation()
            .into_iter()
            .find(|field| field.name() == name)
            .ok_or_else(|| DomainError::RelationFieldNotFound {
                name: name.to_string(),
                model: self.model().name().to_owned(),
            })
    }

    pub fn filter_all<P>(&self, predicate: P) -> Vec<Field>
    where
        P: Fn(&&Field) -> bool,
    {
        let model = self.model();
        let internal_data_model = model.internal_data_model();
        let model_walker = internal_data_model.walk(model.id);
        model_walker
            .scalar_fields()
            .filter(|f| !f.is_ignored() && !f.is_unsupported())
            .map(|w| Field::from((internal_data_model.clone(), w)))
            .chain(
                model_walker
                    .relation_fields()
                    .filter(|rf| !rf.relation().is_ignored())
                    .map(|w| Field::from((internal_data_model.clone(), w))),
            )
            .filter(|f| predicate(&f))
            .collect()
    }
}
