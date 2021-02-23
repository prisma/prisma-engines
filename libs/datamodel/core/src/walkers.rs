//! This module contains convenience functions to easily traverse a Datamodel tree.
//! The most prominent functionality is the pain free navigation of relations.
use crate::{
    dml::{
        Datamodel, DefaultValue, Enum, EnumValue, FieldArity, FieldType, IndexDefinition, Model, ScalarField,
        WithDatabaseName,
    },
    NativeTypeInstance, RelationField,
};
use dml::scalars::ScalarType;
use itertools::Itertools;

/// Iterator over all the models in the schema.
pub fn walk_models(datamodel: &Datamodel) -> impl Iterator<Item = ModelWalker<'_>> + '_ {
    (0..datamodel.models.len()).map(move |model_idx| ModelWalker { datamodel, model_idx })
}

/// Iterator to walk all the scalar fields in the schema, associating them with their parent model.
pub fn walk_scalar_fields(datamodel: &Datamodel) -> impl Iterator<Item = ScalarFieldWalker<'_>> + '_ {
    walk_models(datamodel).flat_map(|model| model.scalar_fields())
}

/// Iterator over all the relations in the schema. Each relation will only occur
/// once.
pub fn walk_relations(datamodel: &Datamodel) -> impl Iterator<Item = RelationWalker<'_>> {
    walk_models(datamodel)
        .flat_map(move |model| model.relation_fields())
        .unique_by(|walker| walker.relation_name())
        .map(|relation_field| {
            let field_b = relation_field.opposite_side();

            RelationWalker {
                field_a: relation_field,
                field_b,
            }
        })
}

/// Find the model mapping to the passed in database name.
pub fn find_model_by_db_name<'a>(datamodel: &'a Datamodel, db_name: &str) -> Option<ModelWalker<'a>> {
    datamodel
        .models
        .iter()
        .enumerate()
        .find(|(_, model)| model.database_name() == Some(db_name) || model.name == db_name)
        .map(|(model_idx, _model)| ModelWalker { datamodel, model_idx })
}

#[derive(Debug, Copy, Clone)]
pub struct ModelWalker<'a> {
    datamodel: &'a Datamodel,
    model_idx: usize,
}

impl<'a> ModelWalker<'a> {
    pub fn database_name(&self) -> &'a str {
        self.get().database_name.as_ref().unwrap_or(&self.get().name)
    }

    pub fn db_name(&self) -> &str {
        self.get().final_database_name()
    }

    fn get(&self) -> &'a Model {
        &self.datamodel.models[self.model_idx]
    }

    /// Return the position of the scalar field with the provided name, if any.
    pub fn index_of_scalar_field(&self, name: &str) -> Option<usize> {
        self.scalar_fields().position(|sf| sf.name() == name)
    }

    pub fn relation_fields(&self) -> impl Iterator<Item = RelationFieldWalker<'a>> + 'a {
        let datamodel = self.datamodel;
        let model_idx = self.model_idx;

        self.get()
            .fields
            .iter()
            .enumerate()
            .filter(|(_idx, field)| field.is_relation())
            .map(move |(field_idx, _)| RelationFieldWalker {
                datamodel,
                model_idx,
                field_idx,
            })
    }

    pub fn scalar_fields(&self) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'a {
        let datamodel = self.datamodel;
        let model_idx = self.model_idx;

        self.get()
            .fields
            .iter()
            .enumerate()
            .filter(|(_idx, field)| field.is_scalar_field())
            .map(move |(field_idx, _)| ScalarFieldWalker {
                datamodel,
                model_idx,
                field_idx,
            })
    }

    pub fn find_scalar_field(&self, name: &str) -> Option<ScalarFieldWalker<'a>> {
        self.scalar_fields().find(|sf| sf.name() == name)
    }

    pub fn indexes<'b>(&'b self) -> impl Iterator<Item = &'a IndexDefinition> + 'b {
        self.get().indices.iter()
    }

    pub fn name(&self) -> &'a str {
        &self.get().name
    }

    pub fn id_fields(&self) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'a {
        let walker = *self;
        let model_idx = self.model_idx;
        let datamodel = self.datamodel;

        self.scalar_fields()
            // Single-id models
            .filter(|field| field.is_id())
            // Compound id models
            .chain(
                self.get()
                    .id_fields
                    .iter()
                    .filter_map(move |field_name| walker.find_scalar_field(field_name)),
            )
            .map(move |field| ScalarFieldWalker {
                datamodel,
                model_idx,
                field_idx: field.field_idx,
            })
    }

    pub fn unique_indexes<'b>(&'b self) -> impl Iterator<Item = IndexWalker<'a>> + 'b {
        self.get()
            .indices
            .iter()
            .filter(|index| index.is_unique())
            .map(move |index| IndexWalker {
                model: *self,
                index,
                datamodel: &self.datamodel,
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScalarFieldWalker<'a> {
    datamodel: &'a Datamodel,
    model_idx: usize,
    field_idx: usize,
}

impl<'a> ScalarFieldWalker<'a> {
    pub fn arity(&self) -> FieldArity {
        self.get().arity
    }

    pub fn db_name(&self) -> &'a str {
        self.get().final_database_name()
    }

    pub fn default_value(&self) -> Option<&'a DefaultValue> {
        self.get().default_value.as_ref()
    }

    pub fn field_index(&self) -> usize {
        self.field_idx
    }

    pub fn field_type(&self) -> TypeWalker<'a> {
        match &self.get().field_type {
            FieldType::Enum(name) => TypeWalker::Enum(EnumWalker {
                datamodel: self.datamodel,
                r#enum: self.datamodel.find_enum(name).unwrap(),
            }),
            FieldType::Base(scalar_type, _) => TypeWalker::Base(*scalar_type),
            FieldType::NativeType(scalar_type, native_type) => TypeWalker::NativeType(*scalar_type, native_type),
            FieldType::Unsupported(description) => TypeWalker::Unsupported(description.clone()),
            FieldType::Relation(_) => unreachable!("FieldType::Relation in ScalarFieldWalker"),
        }
    }

    fn get(&self) -> &'a ScalarField {
        self.datamodel.models[self.model_idx].fields[self.field_idx]
            .as_scalar_field()
            .unwrap()
    }

    pub fn is_id(&self) -> bool {
        self.get().is_id
    }

    pub fn is_required(&self) -> bool {
        self.get().is_required()
    }

    pub fn is_unique(&self) -> bool {
        self.get().is_unique
    }

    pub fn model(&self) -> ModelWalker<'a> {
        ModelWalker {
            model_idx: self.model_idx,
            datamodel: self.datamodel,
        }
    }

    pub fn name(&self) -> &'a str {
        &self.get().name
    }
}

#[derive(Debug)]
pub enum TypeWalker<'a> {
    Enum(EnumWalker<'a>),
    Base(ScalarType),
    NativeType(ScalarType, &'a NativeTypeInstance),
    Unsupported(String),
}

impl<'a> TypeWalker<'a> {
    pub fn as_enum(&self) -> Option<EnumWalker<'a>> {
        match self {
            TypeWalker::Enum(r) => Some(*r),
            _ => None,
        }
    }

    pub fn is_int(&self) -> bool {
        matches!(self, TypeWalker::Base(ScalarType::Int))
    }

    pub fn is_json(&self) -> bool {
        matches!(self, TypeWalker::Base(ScalarType::Json))
    }
}

#[derive(Debug, Clone)]
pub struct RelationFieldWalker<'a> {
    datamodel: &'a Datamodel,
    model_idx: usize,
    field_idx: usize,
}

impl<'a> RelationFieldWalker<'a> {
    pub fn arity(&self) -> FieldArity {
        self.get().arity
    }

    fn get(&self) -> &'a RelationField {
        self.datamodel.models[self.model_idx].fields[self.field_idx]
            .as_relation_field()
            .unwrap()
    }

    pub fn is_one_to_one(&self) -> bool {
        self.get().is_singular() && self.opposite_side().get().is_singular()
    }

    pub fn is_virtual(&self) -> bool {
        self.get().relation_info.fields.is_empty()
    }

    pub fn model(&self) -> ModelWalker<'a> {
        ModelWalker {
            datamodel: self.datamodel,
            model_idx: self.model_idx,
        }
    }

    pub fn opposite_side(&self) -> RelationFieldWalker<'a> {
        RelationFieldWalker {
            datamodel: self.datamodel,
            model_idx: self.referenced_model().model_idx,
            field_idx: self.datamodel.find_related_field_bang(self.get()).0,
        }
    }

    pub fn constrained_field_names(&self) -> &'a [String] {
        &self.get().relation_info.fields
    }

    pub fn constrained_fields(&self) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'a {
        let model_walker = self.model();

        self
            .get()
            .relation_info
            .fields
            .iter()
            .map(move |field| {
                model_walker
                    .find_scalar_field(field.as_str())
                    .unwrap_or_else(|| panic!("Unable to resolve field {} on {}, Expected relation `fields` to point to fields on the enclosing model.", field, model_walker.name()))
            })
    }

    pub fn referenced_columns<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self
            .get()
            .relation_info
            .references
            .iter()
            .map(move |field| {
                let model = self.referenced_model();
                let field = model.find_scalar_field(field.as_str())
                .unwrap_or_else(|| panic!("Unable to resolve field {} on {}, Expected relation `references` to point to fields on the related model.", field, model.name()));

                field.db_name()
            })
    }

    pub fn relation_name(&self) -> &'a str {
        self.get().relation_info.name.as_ref()
    }

    pub fn referenced_model(&self) -> ModelWalker<'a> {
        ModelWalker {
            datamodel: &self.datamodel,
            model_idx: self
                .datamodel
                .models
                .iter()
                .position(|model| model.name == self.get().relation_info.to)
                .unwrap_or_else(|| {
                    panic!(
                        "Invariant violation: could not find model {} referenced in relation info.",
                        self.get().relation_info.to
                    )
                }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnumWalker<'a> {
    pub r#enum: &'a Enum,
    datamodel: &'a Datamodel,
}

impl<'a> EnumWalker<'a> {
    pub fn db_name(&self) -> &'a str {
        self.r#enum.final_database_name()
    }

    pub fn value(&self, name: &str) -> Option<&EnumValue> {
        self.r#enum.values.iter().find(|val| val.name == name)
    }
}

#[derive(Debug)]
pub struct IndexWalker<'a> {
    index: &'a IndexDefinition,
    model: ModelWalker<'a>,
    datamodel: &'a Datamodel,
}

impl<'a> IndexWalker<'a> {
    pub fn fields<'b>(&'b self) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'b {
        self.index.fields.iter().map(move |field_name| {
            self.model
                .scalar_fields()
                .find(|f| f.name() == field_name.as_str())
                .expect("index on unknown model field")
        })
    }

    pub fn is_unique(&self) -> bool {
        self.index.is_unique()
    }
}

#[derive(Debug)]
pub struct RelationWalker<'a> {
    field_a: RelationFieldWalker<'a>,
    field_b: RelationFieldWalker<'a>,
}

impl<'a> RelationWalker<'a> {
    pub fn as_m2m(&self) -> Option<ManyToManyRelationWalker<'a>> {
        match (self.field_a.arity(), self.field_b.arity()) {
            (FieldArity::List, FieldArity::List) => {
                let (field_a, field_b) = if self.field_a.model().name() < self.field_b.model().name() {
                    (&self.field_a, &self.field_b)
                } else {
                    (&self.field_b, &self.field_a)
                };

                Some(ManyToManyRelationWalker {
                    field_a: field_a.clone(),
                    field_b: field_b.clone(),
                })
            }
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ManyToManyRelationWalker<'a> {
    field_a: RelationFieldWalker<'a>,
    field_b: RelationFieldWalker<'a>,
}

impl<'a> ManyToManyRelationWalker<'a> {
    pub const fn model_a_column(&self) -> &str {
        "A"
    }

    pub fn model_a_id(&self) -> ScalarFieldWalker<'a> {
        self.field_a
            .model()
            .id_fields()
            .next()
            .expect("Missing id field on a model in a M2M relation.")
    }

    pub const fn model_b_column(&self) -> &str {
        "B"
    }

    pub fn model_b_id(&self) -> ScalarFieldWalker<'a> {
        self.field_b
            .model()
            .id_fields()
            .next()
            .expect("Missing id field on a model in a M2M relation.")
    }

    pub fn relation_name(&self) -> &str {
        self.field_a.relation_name()
    }

    pub fn table_name(&self) -> String {
        format!("_{}", self.relation_name())
    }
}
