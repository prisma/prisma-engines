use crate::{ModelRef, RelationFieldRef, ScalarFieldRef, TypeIdentifier};
use datamodel::FieldArity;

pub trait IntoSelectedFields {
    fn into_selected_fields(self, model: ModelRef) -> SelectedFields;
}

#[derive(Debug, Default, Clone)]
pub struct SelectedFields {
    pub scalar: Vec<SelectedScalarField>,
    pub relation: Vec<SelectedRelationField>,
}

#[derive(Debug, Clone)]
pub enum SelectedField {
    Scalar(SelectedScalarField),
    Relation(SelectedRelationField),
}

#[derive(Debug, Clone)]
pub struct SelectedScalarField {
    pub field: ScalarFieldRef,
}

#[derive(Debug, Clone)]
pub struct SelectedRelationField {
    pub field: RelationFieldRef,
    pub selected_fields: SelectedFields,
}

impl From<ScalarFieldRef> for SelectedField {
    fn from(sf: ScalarFieldRef) -> SelectedField {
        SelectedField::Scalar(SelectedScalarField { field: sf })
    }
}

impl From<ScalarFieldRef> for SelectedFields {
    fn from(sf: ScalarFieldRef) -> SelectedFields {
        SelectedFields::new(vec![SelectedField::from(sf)])
    }
}

impl From<Vec<ScalarFieldRef>> for SelectedFields {
    fn from(sfs: Vec<ScalarFieldRef>) -> SelectedFields {
        let fields = sfs.into_iter().map(SelectedField::from).collect();

        SelectedFields::new(fields)
    }
}

impl From<&ModelRef> for SelectedFields {
    fn from(model: &ModelRef) -> SelectedFields {
        let fields = model.fields().scalar().into_iter().map(SelectedField::from).collect();

        SelectedFields::new(fields)
    }
}

impl SelectedFields {
    pub const RELATED_MODEL_ALIAS: &'static str = "__RelatedModel__";
    pub const PARENT_MODEL_ALIAS: &'static str = "__ParentModel__";

    pub fn new(fields: Vec<SelectedField>) -> SelectedFields {
        let (scalar, relation) = fields.into_iter().fold((Vec::new(), Vec::new()), |mut acc, field| {
            match field {
                SelectedField::Scalar(sf) => acc.0.push(sf),
                SelectedField::Relation(sf) => acc.1.push(sf),
            }

            acc
        });

        SelectedFields { scalar, relation }
    }

    pub fn id(model: ModelRef) -> Self {
        Self::from(model.fields().id())
    }

    pub fn add_scalar(&mut self, field: ScalarFieldRef) {
        self.scalar.push(SelectedScalarField { field });
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        let scalar = self.scalar_fields().map(|f| f.name.as_str());
        let relation = self.relation_inlined().map(|f| f.name.as_str());

        scalar.chain(relation)
    }

    pub fn types<'a>(&'a self) -> impl Iterator<Item = (TypeIdentifier, FieldArity)> + 'a {
        let scalar = self.scalar_fields().map(|sf| sf.type_identifier_with_arity());
        let relation = self.relation_inlined().map(|rf| rf.type_identifier_with_arity());

        scalar.chain(relation)
    }

    pub fn model(&self) -> ModelRef {
        self.scalar
            .first()
            .map(|s| s.field.model())
            .or_else(|| self.relation.first().map(|r| r.field.model()))
            .expect("Expected at least one field to be present.")
    }

    pub(super) fn relation_inlined(&self) -> impl Iterator<Item = &RelationFieldRef> {
        self.relation.iter().map(|rf| &rf.field).filter(|rf| {
            let relation = rf.relation();
            let is_inline = relation.is_inline_relation();
            let is_self = relation.is_self_relation();

            let is_intable = relation
                .inline_manifestation()
                .map(|mf| mf.in_table_of_model_name == rf.model().name)
                .unwrap_or(false);

            (is_inline && is_self && rf.relation_side.is_b()) || (is_inline && !is_self && is_intable)
        })
    }

    pub(super) fn scalar_fields(&self) -> impl Iterator<Item = &ScalarFieldRef> {
        self.scalar.iter().map(|sf| &sf.field)
    }
}
