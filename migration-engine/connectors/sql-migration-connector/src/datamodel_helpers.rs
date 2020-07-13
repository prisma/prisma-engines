use datamodel::{
    dml::{
        Datamodel, DefaultValue, Enum, FieldArity, FieldType, IndexDefinition, Model, ScalarField, ScalarType,
        WithDatabaseName,
    },
    RelationField,
};

pub(crate) fn walk_models<'a>(datamodel: &'a Datamodel) -> impl Iterator<Item = ModelRef<'a>> + 'a {
    datamodel.models.iter().map(move |model| ModelRef { datamodel, model })
}

/// Iterator to walk all the scalar fields in the schema, associating them with their parent model.
pub(super) fn walk_scalar_fields<'a>(datamodel: &'a Datamodel) -> impl Iterator<Item = ScalarFieldRef<'a>> + 'a {
    datamodel.models().flat_map(move |model| {
        model.scalar_fields().map(move |field| ScalarFieldRef {
            datamodel,
            model,
            field,
        })
    })
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct ModelRef<'a> {
    datamodel: &'a Datamodel,
    model: &'a Model,
}

impl<'a> ModelRef<'a> {
    pub(crate) fn new(model: &'a Model, datamodel: &'a Datamodel) -> Self {
        ModelRef { datamodel, model }
    }

    pub(super) fn database_name(&self) -> &'a str {
        self.model.database_name.as_ref().unwrap_or(&self.model.name)
    }

    pub(super) fn db_name(&self) -> &str {
        self.model.final_database_name()
    }

    pub(super) fn relation_fields<'b>(&'b self) -> impl Iterator<Item = RelationFieldRef<'a>> + 'b {
        self.model.relation_fields().map(move |field| RelationFieldRef {
            datamodel: self.datamodel,
            model: self.model,
            field,
        })
    }

    pub(super) fn scalar_fields<'b>(&'b self) -> impl Iterator<Item = ScalarFieldRef<'a>> + 'b {
        self.model.scalar_fields().map(move |field| ScalarFieldRef {
            datamodel: self.datamodel,
            model: self.model,
            field,
        })
    }

    pub(super) fn find_scalar_field(&self, name: &str) -> Option<ScalarFieldRef<'a>> {
        self.model.find_scalar_field(name).map(|field| ScalarFieldRef {
            datamodel: self.datamodel,
            field,
            model: self.model,
        })
    }

    pub(super) fn indexes<'b>(&'b self) -> impl Iterator<Item = &'a IndexDefinition> + 'b {
        self.model.indices.iter()
    }

    pub(super) fn name(&self) -> &'a str {
        &self.model.name
    }

    pub(super) fn id_fields<'b>(&'b self) -> impl Iterator<Item = ScalarFieldRef<'a>> + 'b {
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
            .map(move |field| ScalarFieldRef {
                datamodel: self.datamodel,
                model: self.model,
                field,
            })
    }

    pub(super) fn unique_indexes<'b>(&'b self) -> impl Iterator<Item = IndexRef<'a>> + 'b {
        self.model
            .indices
            .iter()
            .filter(|index| index.is_unique())
            .map(move |index| IndexRef {
                model: *self,
                index,
                datamodel: &self.datamodel,
            })
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ScalarFieldRef<'a> {
    datamodel: &'a Datamodel,
    model: &'a Model,
    field: &'a ScalarField,
}

impl<'a> ScalarFieldRef<'a> {
    pub(super) fn arity(&self) -> FieldArity {
        self.field.arity
    }

    pub(super) fn db_name(&self) -> &'a str {
        self.field.final_database_name()
    }

    pub(super) fn default_value(&self) -> Option<&'a DefaultValue> {
        self.field.default_value.as_ref()
    }

    pub(super) fn field_type(&self) -> TypeRef<'a> {
        match &self.field.field_type {
            FieldType::Enum(name) => TypeRef::Enum(EnumRef {
                datamodel: self.datamodel,
                r#enum: self.datamodel.find_enum(name).unwrap(),
            }),
            FieldType::Base(scalar_type, _) => TypeRef::Base(*scalar_type),
            _ => TypeRef::Other,
        }
    }

    pub(super) fn is_id(&self) -> bool {
        self.field.is_id
    }

    pub(super) fn is_required(&self) -> bool {
        match self.arity() {
            FieldArity::Required => true,
            _ => false,
        }
    }

    pub(super) fn is_unique(&self) -> bool {
        self.field.is_unique
    }

    pub(super) fn model(&self) -> ModelRef<'a> {
        ModelRef {
            model: self.model,
            datamodel: self.datamodel,
        }
    }

    pub(super) fn name(&self) -> &'a str {
        &self.field.name
    }
}

#[derive(Debug)]
pub(super) enum TypeRef<'a> {
    Enum(EnumRef<'a>),
    Base(ScalarType),
    Other,
}

impl<'a> TypeRef<'a> {
    pub(super) fn as_enum(&self) -> Option<EnumRef<'a>> {
        match self {
            TypeRef::Enum(r) => Some(*r),
            _ => None,
        }
    }

    pub(super) fn is_json(&self) -> bool {
        matches!(self, TypeRef::Base(ScalarType::Json))
    }
}

#[derive(Debug)]
pub(super) struct RelationFieldRef<'a> {
    datamodel: &'a Datamodel,
    model: &'a Model,
    field: &'a RelationField,
}

impl<'a> RelationFieldRef<'a> {
    pub(super) fn arity(&self) -> FieldArity {
        self.field.arity
    }

    pub(crate) fn is_one_to_one(&self) -> bool {
        self.field.is_singular() && self.opposite_side().map(|rel| rel.field.is_singular()).unwrap_or(false)
    }

    pub(crate) fn is_virtual(&self) -> bool {
        self.field.relation_info.fields.is_empty()
    }

    pub(crate) fn opposite_side(&self) -> Option<RelationFieldRef<'a>> {
        self.referenced_model_ref().relation_fields().find(|relation_field| {
            relation_field.relation_name() == self.relation_name()
                    && relation_field.referenced_model().name.as_str() == self.model.name
                    // This is to differentiate the opposite field from self in the self relation case.
                    && relation_field.field.relation_info.to_fields != self.field.relation_info.to_fields
                    && relation_field.field.relation_info.fields != self.field.relation_info.fields
        })
    }

    pub(crate) fn referencing_columns<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
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

    pub(crate) fn referenced_columns<'b>(&'b self) -> impl Iterator<Item = &'a str> + 'b {
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

    pub(crate) fn relation_name(&self) -> &'a str {
        self.field.relation_info.name.as_ref()
    }

    pub(crate) fn referenced_table_name(&self) -> &'a str {
        self.referenced_model().final_database_name()
    }

    fn referenced_model(&self) -> &'a Model {
        self.datamodel
            .find_model(&self.field.relation_info.to)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invariant violation: could not find model {} referenced in relation info.",
                    self.field.relation_info.to
                )
            })
            .unwrap()
    }

    fn referenced_model_ref(&self) -> ModelRef<'a> {
        ModelRef {
            model: self.referenced_model(),
            datamodel: self.datamodel,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EnumRef<'a> {
    pub(super) r#enum: &'a Enum,
    datamodel: &'a Datamodel,
}

impl<'a> EnumRef<'a> {
    pub(super) fn db_name(&self) -> &'a str {
        self.r#enum.final_database_name()
    }
}

#[derive(Debug)]
pub(super) struct IndexRef<'a> {
    index: &'a IndexDefinition,
    model: ModelRef<'a>,
    datamodel: &'a Datamodel,
}

impl<'a> IndexRef<'a> {
    pub(super) fn fields<'b>(&'b self) -> impl Iterator<Item = ScalarFieldRef<'a>> + 'b {
        self.index.fields.iter().map(move |field_name| {
            self.model
                .scalar_fields()
                .find(|f| f.name() == field_name.as_str())
                .expect("index on unknown model field")
        })
    }
}
