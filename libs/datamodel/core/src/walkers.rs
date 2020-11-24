//! This module contains convenience functions to easily traverse a Datamodel tree.
//! The most prominent functionality is the pain free navigation of relations.
use crate::{
    dml::{
        Datamodel, DefaultValue, Enum, FieldArity, FieldType, IndexDefinition, Model, ScalarField, WithDatabaseName,
    },
    NativeTypeInstance, RelationField,
};
use dml::scalars::ScalarType;
use itertools::Itertools;

/// Iterator over all the models in the schema.
pub fn walk_models<'a>(datamodel: &'a Datamodel) -> impl Iterator<Item = ModelWalker<'a>> + 'a {
    datamodel.models().map(move |model| ModelWalker { datamodel, model })
}

/// Iterator to walk all the scalar fields in the schema, associating them with their parent model.
pub fn walk_scalar_fields<'a>(datamodel: &'a Datamodel) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'a {
    datamodel.models().flat_map(move |model| {
        model.scalar_fields().map(move |field| ScalarFieldWalker {
            datamodel,
            model,
            field,
        })
    })
}

/// Iterator over all the relations in the schema. Each relation will only occur
/// once.
pub fn walk_relations(datamodel: &Datamodel) -> impl Iterator<Item = RelationWalker<'_>> {
    walk_models(datamodel)
        .flat_map(move |model| model.into_relation_fields())
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
        .models()
        .find(|model| model.database_name() == Some(db_name) || model.name == db_name)
        .map(|model| ModelWalker { datamodel, model })
}

#[derive(Debug, Copy, Clone)]
pub struct ModelWalker<'a> {
    datamodel: &'a Datamodel,
    model: &'a Model,
}

impl<'a> ModelWalker<'a> {
    pub fn new(model: &'a Model, datamodel: &'a Datamodel) -> Self {
        ModelWalker { datamodel, model }
    }

    pub fn database_name(&self) -> &'a str {
        self.model.database_name.as_ref().unwrap_or(&self.model.name)
    }

    pub fn db_name(&self) -> &str {
        self.model.final_database_name()
    }

    pub fn into_relation_fields(self) -> impl Iterator<Item = RelationFieldWalker<'a>> + 'a {
        self.model.relation_fields().map(move |field| RelationFieldWalker {
            datamodel: self.datamodel,
            model: self.model,
            field,
        })
    }

    pub fn relation_fields<'b>(&'b self) -> impl Iterator<Item = RelationFieldWalker<'a>> + 'b {
        self.model.relation_fields().map(move |field| RelationFieldWalker {
            datamodel: self.datamodel,
            model: self.model,
            field,
        })
    }

    pub fn scalar_fields<'b>(&'b self) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'b {
        self.model.scalar_fields().map(move |field| ScalarFieldWalker {
            datamodel: self.datamodel,
            model: self.model,
            field,
        })
    }

    pub fn find_scalar_field(&self, name: &str) -> Option<ScalarFieldWalker<'a>> {
        self.model.find_scalar_field(name).map(|field| ScalarFieldWalker {
            datamodel: self.datamodel,
            field,
            model: self.model,
        })
    }

    pub fn indexes<'b>(&'b self) -> impl Iterator<Item = &'a IndexDefinition> + 'b {
        self.model.indices.iter()
    }

    pub fn name(&self) -> &'a str {
        &self.model.name
    }

    pub fn id_fields<'b>(&'b self) -> impl Iterator<Item = ScalarFieldWalker<'a>> + 'b {
        // Single-id models
        self.model
            .scalar_fields()
            .filter(|field| field.is_id)
            // Compound id models
            .chain(
                self.model
                    .id_fields
                    .iter()
                    .filter_map(move |field_name| self.model.find_scalar_field(field_name)),
            )
            .map(move |field| ScalarFieldWalker {
                datamodel: self.datamodel,
                model: self.model,
                field,
            })
    }

    pub fn unique_indexes<'b>(&'b self) -> impl Iterator<Item = IndexWalker<'a>> + 'b {
        self.model
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
    model: &'a Model,
    field: &'a ScalarField,
}

impl<'a> ScalarFieldWalker<'a> {
    pub fn arity(&self) -> FieldArity {
        self.field.arity
    }

    pub fn db_name(&self) -> &'a str {
        self.field.final_database_name()
    }

    pub fn default_value(&self) -> Option<&'a DefaultValue> {
        self.field.default_value.as_ref()
    }

    pub fn field_type(&self) -> TypeWalker<'a> {
        match &self.field.field_type {
            FieldType::Enum(name) => TypeWalker::Enum(EnumWalker {
                datamodel: self.datamodel,
                r#enum: self.datamodel.find_enum(name).unwrap(),
            }),
            FieldType::Base(scalar_type, _) => TypeWalker::Base(*scalar_type),
            FieldType::NativeType(scalar_type, native_type) => TypeWalker::NativeType(*scalar_type, native_type),
            _ => TypeWalker::Other,
        }
    }

    pub fn is_id(&self) -> bool {
        self.field.is_id
    }

    pub fn is_required(&self) -> bool {
        self.field.is_required()
    }

    pub fn is_unique(&self) -> bool {
        self.field.is_unique
    }

    pub fn model(&self) -> ModelWalker<'a> {
        ModelWalker {
            model: self.model,
            datamodel: self.datamodel,
        }
    }

    pub fn name(&self) -> &'a str {
        &self.field.name
    }
}

#[derive(Debug)]
pub enum TypeWalker<'a> {
    Enum(EnumWalker<'a>),
    Base(ScalarType),
    NativeType(ScalarType, &'a NativeTypeInstance),
    Other,
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
    model: &'a Model,
    field: &'a RelationField,
}

impl<'a> RelationFieldWalker<'a> {
    pub fn arity(&self) -> FieldArity {
        self.field.arity
    }

    pub fn is_one_to_one(&self) -> bool {
        self.field.is_singular() && self.opposite_side().field.is_singular()
    }

    pub fn is_virtual(&self) -> bool {
        self.field.relation_info.fields.is_empty()
    }

    pub fn model(&self) -> ModelWalker<'a> {
        ModelWalker {
            datamodel: self.datamodel,
            model: self.model,
        }
    }

    pub fn opposite_side(&self) -> RelationFieldWalker<'a> {
        RelationFieldWalker {
            datamodel: self.datamodel,
            model: self.referenced_model().model,
            field: self.datamodel.find_related_field_bang(self.field),
        }
    }

    pub fn referencing_columns<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self
            .field
            .relation_info
            .fields
            .iter()
            .map(move |field| {
                let field = self.model.find_scalar_field(field.as_str())
                .unwrap_or_else(|| panic!("Unable to resolve field {} on {}, Expected relation `fields` to point to fields on the enclosing model.", field, self.model.name));

                field.db_name()
            })
    }

    pub fn referenced_columns<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
        self
            .field
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
        self.field.relation_info.name.as_ref()
    }

    pub fn referenced_model(&self) -> ModelWalker<'a> {
        ModelWalker {
            datamodel: &self.datamodel,
            model: self
                .datamodel
                .find_model(&self.field.relation_info.to)
                .unwrap_or_else(|| {
                    panic!(
                        "Invariant violation: could not find model {} referenced in relation info.",
                        self.field.relation_info.to
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
