use crate::{Field, ModelIdentifier, ModelRef, RelationFieldRef, ScalarFieldRef, TypeIdentifier};
use datamodel::FieldArity;
use itertools::Itertools;

pub trait IntoSelectedFields {
    fn into_selected_fields(self, model: ModelRef) -> SelectedFields;
}

#[derive(Debug, Default, Clone, PartialEq, Hash, Eq)]
pub struct SelectedFields {
    pub scalar: Vec<SelectedScalarField>,
    pub relation: Vec<SelectedRelationField>,
}
impl SelectedFields {
    // [DTODO] Remove
    pub fn only_scalar_and_inlined(&self) -> SelectedFields {
        SelectedFields {
            scalar: self.scalar.clone(),
            relation: self
                .relation
                .iter()
                .filter_map(|x| {
                    if x.field.is_inlined_on_enclosing_model() {
                        Some(SelectedRelationField { field: x.field.clone() })
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelectedField {
    Scalar(SelectedScalarField),
    Relation(SelectedRelationField),
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct SelectedScalarField {
    pub field: ScalarFieldRef,
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct SelectedRelationField {
    pub field: RelationFieldRef,
}

impl From<Field> for SelectedField {
    fn from(field: Field) -> SelectedField {
        match field {
            Field::Scalar(sf) => sf.into(),
            Field::Relation(rf) => rf.into(),
        }
    }
}

impl From<RelationFieldRef> for SelectedField {
    fn from(field: RelationFieldRef) -> SelectedField {
        SelectedField::Relation(SelectedRelationField { field })
    }
}

impl From<ScalarFieldRef> for SelectedField {
    fn from(field: ScalarFieldRef) -> SelectedField {
        SelectedField::Scalar(SelectedScalarField { field })
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

impl From<Vec<Field>> for SelectedFields {
    fn from(fields: Vec<Field>) -> SelectedFields {
        let fields = fields
            .into_iter()
            .map(|f| match f {
                Field::Scalar(sf) => SelectedField::from(sf),
                Field::Relation(rf) => SelectedField::from(rf),
            })
            .collect();

        SelectedFields::new(fields)
    }
}

impl From<ModelIdentifier> for SelectedFields {
    fn from(id: ModelIdentifier) -> SelectedFields {
        let fields = id.into_iter().map(SelectedField::from).collect();
        SelectedFields::new(fields)
    }
}

impl From<Vec<ModelIdentifier>> for SelectedFields {
    fn from(ids: Vec<ModelIdentifier>) -> SelectedFields {
        let fields = ids
            .into_iter()
            .flat_map(|id| id.into_iter().map(SelectedField::from).collect::<Vec<_>>())
            .collect();

        SelectedFields::new(fields).deduplicate()
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

    pub fn add(&mut self, field: Field) {
        match field {
            Field::Scalar(sf) => self.add_scalar(sf),
            Field::Relation(rf) => self.add_relation(rf),
        }
    }

    pub fn add_scalar(&mut self, field: ScalarFieldRef) {
        self.scalar.push(SelectedScalarField { field });
    }

    pub fn add_relation(&mut self, field: RelationFieldRef) {
        self.relation.push(SelectedRelationField { field });
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        let scalar = self.scalar_fields().map(|f| f.name.as_str());
        let relation = self.relation_fields().map(|f| f.name.as_str());

        scalar.chain(relation)
    }

    pub fn db_names(&self) -> impl Iterator<Item = &str> {
        let scalar = self.scalar_fields().map(|f| f.data_source_field().name.as_str());
        let relation = self
            .relation_fields()
            .flat_map(|f| f.data_source_fields().into_iter().map(|dsf| dsf.name.as_str()));

        scalar.chain(relation)
    }

    pub fn types<'a>(&'a self) -> impl Iterator<Item = (TypeIdentifier, FieldArity)> + 'a {
        let scalar = self.scalar_fields().map(|sf| sf.type_identifier_with_arity());
        let relation = self.relation_fields().flat_map(|rf| rf.type_identifiers_with_arities());

        scalar.chain(relation)
    }

    pub fn model(&self) -> ModelRef {
        self.scalar
            .first()
            .map(|s| s.field.model())
            .or_else(|| self.relation.first().map(|r| r.field.model()))
            .expect("Expected at least one field to be present.")
    }

    pub(super) fn scalar_fields(&self) -> impl Iterator<Item = &ScalarFieldRef> {
        self.scalar.iter().map(|sf| &sf.field)
    }

    pub(super) fn relation_fields(&self) -> impl Iterator<Item = &RelationFieldRef> {
        self.relation.iter().map(|rf| &rf.field)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names().find(|fname| fname == &name).is_some()
    }

    pub fn contains_all_db_names<'a>(&self, names: impl Iterator<Item = String>) -> bool {
        let selected_db_names: Vec<_> = self.db_names().collect();
        let names_to_select: Vec<_> = names.collect();

        dbg!(&selected_db_names);
        dbg!(&names_to_select);

        if names_to_select.len() > selected_db_names.len() {
            false
        } else {
            names_to_select
                .into_iter()
                .all(|to_select| selected_db_names.contains(&to_select.as_str()))
        }
    }

    pub fn deduplicate(mut self) -> Self {
        self.scalar = self.scalar.into_iter().unique().collect();
        self.relation = self.relation.into_iter().unique().collect();
        self
    }
}
