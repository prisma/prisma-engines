use crate::{ModelRef, RelationField, ScalarField, TypeIdentifier};
use datamodel::FieldArity;
use std::sync::Arc;

pub trait IntoSelectedFields {
    fn into_selected_fields(self, model: ModelRef, from_field: Option<Arc<RelationField>>) -> SelectedFields;
}

#[derive(Debug, Default, Clone)]
pub struct SelectedFields {
    pub scalar: Vec<SelectedScalarField>,
    pub relation: Vec<SelectedRelationField>,

    /// FIXME: naming
    pub from_field: Option<Arc<RelationField>>,
}

#[derive(Debug, Clone)]
pub enum SelectedField {
    Scalar(SelectedScalarField),
    Relation(SelectedRelationField),
}

#[derive(Debug, Clone)]
pub struct SelectedScalarField {
    pub field: Arc<ScalarField>,
}

#[derive(Debug, Clone)]
pub struct SelectedRelationField {
    pub field: Arc<RelationField>,
    pub selected_fields: SelectedFields,
}

impl From<Arc<ScalarField>> for SelectedField {
    fn from(sf: Arc<ScalarField>) -> SelectedField {
        SelectedField::Scalar(SelectedScalarField { field: sf })
    }
}

impl From<Arc<ScalarField>> for SelectedFields {
    fn from(sf: Arc<ScalarField>) -> SelectedFields {
        SelectedFields::new(vec![SelectedField::from(sf)], None)
    }
}

impl From<Vec<Arc<ScalarField>>> for SelectedFields {
    fn from(sfs: Vec<Arc<ScalarField>>) -> SelectedFields {
        let fields = sfs.into_iter().map(SelectedField::from).collect();

        SelectedFields::new(fields, None)
    }
}

impl From<&ModelRef> for SelectedFields {
    fn from(model: &ModelRef) -> SelectedFields {
        let fields = model.fields().scalar().into_iter().map(SelectedField::from).collect();

        SelectedFields::new(fields, None)
    }
}

impl SelectedFields {
    pub const RELATED_MODEL_ALIAS: &'static str = "__RelatedModel__";
    pub const PARENT_MODEL_ALIAS: &'static str = "__ParentModel__";

    pub fn new(fields: Vec<SelectedField>, from_field: Option<Arc<RelationField>>) -> SelectedFields {
        let (scalar, relation) = fields.into_iter().fold((Vec::new(), Vec::new()), |mut acc, field| {
            match field {
                SelectedField::Scalar(sf) => acc.0.push(sf),
                SelectedField::Relation(sf) => acc.1.push(sf),
            }

            acc
        });

        SelectedFields {
            scalar,
            relation,
            from_field,
        }
    }

    pub fn id(model: ModelRef) -> Self {
        Self::from(model.fields().id())
    }

    pub fn add_scalar(&mut self, field: Arc<ScalarField>) {
        self.scalar.push(SelectedScalarField { field });
    }

    pub fn names(&self) -> Vec<String> {
        let mut result: Vec<String> = self.scalar_fields().iter().map(|f| f.name.clone()).collect();

        for rf in self.relation_inlined().iter() {
            result.push(rf.name.clone());
        }

        if let Some(ref from_field) = self.from_field {
            result.push(from_field.related_field().name.clone());
            result.push(from_field.name.clone());
        };

        result
    }

    pub fn types(&self) -> Vec<(TypeIdentifier, FieldArity)> {
        let mut result: Vec<(TypeIdentifier, FieldArity)> = self
            .scalar_fields()
            .iter()
            .map(|sf| sf.type_identifier_with_arity())
            .collect();

        for rf in self.relation_inlined().iter() {
            result.push(rf.type_identifier_with_arity());
        }

        match &self.from_field {
            Some(rf) => {
                result.push(rf.type_identifier_with_arity());
                result.push(rf.related_field().type_identifier_with_arity());
            }
            None => (),
        }

        result
    }

    pub fn model(&self) -> ModelRef {
        self.scalar
            .first()
            .map(|s| s.field.model())
            .or_else(|| self.relation.first().map(|r| r.field.model()))
            .expect("Expected at least one field to be present.")
    }

    pub fn relation_inlined(&self) -> Vec<Arc<RelationField>> {
        self.relation
            .iter()
            .map(|rf| Arc::clone(&rf.field))
            .filter(|rf| {
                let relation = rf.relation();
                let related = rf.related_field();
                let is_inline = relation.is_inline_relation();
                let is_self = relation.is_self_relation();

                let is_intable = relation
                    .inline_manifestation()
                    .map(|mf| mf.in_table_of_model_name == rf.model().name)
                    .unwrap_or(false);

                (!rf.is_hidden && is_inline && is_self && rf.relation_side.is_b())
                    || (related.is_hidden && is_inline && is_self && rf.relation_side.is_a())
                    || (is_inline && !is_self && is_intable)
            })
            .collect()
    }

    pub fn scalar_fields(&self) -> Vec<Arc<ScalarField>> {
        self.scalar.iter().map(|sf| sf.field.clone()).collect()
    }
}
