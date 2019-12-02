use crate::{ModelRef, RelationFieldRef, ScalarFieldRef, TypeIdentifier};

pub trait IntoSelectedFields {
    fn into_selected_fields(self, model: ModelRef) -> SelectedFields;
}

#[derive(Debug, Default, Clone)]
pub struct SelectedFields {
    fields: Vec<SelectedField>,
}

#[derive(Debug, Clone)]
pub enum SelectedField {
    Scalar(ScalarFieldRef),
    Relation(RelationFieldRef),
}

impl From<ScalarFieldRef> for SelectedField {
    fn from(sf: ScalarFieldRef) -> SelectedField {
        SelectedField::Scalar(sf)
    }
}

impl From<RelationFieldRef> for SelectedField {
    fn from(rf: RelationFieldRef) -> SelectedField {
        SelectedField::Relation(rf)
    }
}

impl From<ScalarFieldRef> for SelectedFields {
    fn from(sf: ScalarFieldRef) -> SelectedFields {
        SelectedFields::new(vec![sf])
    }
}

impl From<Vec<ScalarFieldRef>> for SelectedFields {
    fn from(sfs: Vec<ScalarFieldRef>) -> SelectedFields {
        SelectedFields::new(sfs.into_iter().map(SelectedField::from))
    }
}

impl From<&ModelRef> for SelectedFields {
    fn from(model: &ModelRef) -> SelectedFields {
        let fields = model
            .fields()
            .scalar_non_list()
            .into_iter()
            .map(SelectedField::from);

        SelectedFields::new(fields)
    }
}

impl SelectedFields {
    pub fn new<F>(fields: impl IntoIterator<Item = F>) -> SelectedFields
    where
        F: Into<SelectedField>,
    {
        SelectedFields {
            fields: fields.into_iter().map(Into::into).collect(),
        }
    }

    pub fn id(model: ModelRef) -> Self {
        Self::from(model.fields().id())
    }

    pub fn push<F>(&mut self, field: F)
    where
        F: Into<SelectedField>,
    {
        self.fields.push(field.into());
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        let scalars = self.scalar_non_list().map(|f| f.name.as_str());
        let rels = self.relation_inlined().map(|f| f.name.as_str());

        scalars.chain(rels)
    }

    pub fn type_identifiers(&self) -> impl Iterator<Item = TypeIdentifier> + '_ {
        let scalars = self.scalar_non_list().map(|sf| sf.type_identifier);
        let rels = self.relation_inlined().map(|rf| rf.type_identifier);

        scalars.chain(rels)
    }

    pub fn scalar_non_list(&self) -> impl Iterator<Item = &ScalarFieldRef> {
        self.scalar().filter(|sf| !sf.is_list)
    }

    pub fn scalar_lists(&self) -> impl Iterator<Item = &ScalarFieldRef> {
        self.scalar().filter(|sf| sf.is_list)
    }

    pub(super) fn relation_inlined(&self) -> impl Iterator<Item = &RelationFieldRef> {
        self.relation().filter(|rf| {
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
    }

    fn scalar(&self) -> impl Iterator<Item = &ScalarFieldRef> {
        self.fields.iter().filter_map(|f| {
            if let SelectedField::Scalar(ref sf) = f {
                Some(sf)
            } else {
                None
            }
        })
    }

    fn relation(&self) -> impl Iterator<Item = &RelationFieldRef> {
        self.fields.iter().filter_map(|f| {
            if let SelectedField::Relation(ref rf) = f {
                Some(rf)
            } else {
                None
            }
        })
    }
}
