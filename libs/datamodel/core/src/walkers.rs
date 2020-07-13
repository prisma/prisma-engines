use crate::{
    dml::{
        Datamodel, DefaultValue, Enum, FieldArity, FieldType, IndexDefinition, Model, ScalarField, ScalarType,
        WithDatabaseName,
    },
    RelationField,
};

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
    Other,
}

impl<'a> TypeWalker<'a> {
    pub fn as_enum(&self) -> Option<EnumWalker<'a>> {
        match self {
            TypeWalker::Enum(r) => Some(*r),
            _ => None,
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(self, TypeWalker::Base(ScalarType::Json))
    }
}

#[derive(Debug)]
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

    pub fn opposite_side(&self) -> RelationFieldWalker<'a> {
        RelationFieldWalker {
            datamodel: self.datamodel,
            model: self.referenced_model(),
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
            .to_fields
            .iter()
            .map(move |field| {
                let model = self.referenced_model();
                let field = model.find_scalar_field(field.as_str())
                .unwrap_or_else(|| panic!("Unable to resolve field {} on {}, Expected relation `references` to point to fields on the related model.", field, model.name));

                field.db_name()
            })
    }

    pub fn relation_name(&self) -> &'a str {
        self.field.relation_info.name.as_ref()
    }

    pub fn referenced_table_name(&self) -> &'a str {
        self.referenced_model().final_database_name()
    }

    fn referenced_model(&self) -> &'a Model {
        self.datamodel
            .find_model(&self.field.relation_info.to)
            .unwrap_or_else(|| {
                panic!(
                    "Invariant violation: could not find model {} referenced in relation info.",
                    self.field.relation_info.to
                )
            })
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
}
