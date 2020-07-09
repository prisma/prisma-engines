use super::*;

/// Represents a prisma-datamodel.
#[derive(Debug, PartialEq, Clone)]
pub struct Datamodel {
    /// All enums.
    pub enums: Vec<Enum>,
    /// All models.
    pub models: Vec<Model>,
}

/// Type alias for (ModelName, FieldName)
pub type FieldRef = (String, String);

impl Datamodel {
    /// Creates a new, empty schema.
    pub fn new() -> Datamodel {
        Datamodel {
            enums: Vec::new(),
            models: Vec::new(),
        }
    }

    /// Creates a new, empty schema.
    pub fn empty() -> Datamodel {
        Self::new()
    }

    /// Checks if a model with the given name exists.
    pub fn has_model(&self, name: &str) -> bool {
        self.find_model(name).is_some()
    }

    /// Checks if an enum with the given name exists.
    pub fn has_enum(&self, name: &str) -> bool {
        self.find_enum(name).is_some()
    }

    /// Adds an enum to this datamodel.
    pub fn add_enum(&mut self, en: Enum) {
        self.enums.push(en);
    }

    /// Removes an enum from this datamodel.
    pub fn remove_enum(&mut self, name: &str) {
        self.enums.retain(|m| m.name != name);
    }

    /// Adds a model to this datamodel.
    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);
    }

    /// Removes a model from this datamodel.
    pub fn remove_model(&mut self, name: &str) {
        self.models.retain(|m| m.name != name);
    }

    /// Gets an iterator over all models.
    pub fn models(&self) -> std::slice::Iter<Model> {
        self.models.iter()
    }

    /// Gets an iterator over all enums.
    pub fn enums(&self) -> std::slice::Iter<Enum> {
        self.enums.iter()
    }

    /// Gets a mutable iterator over all models.
    pub fn models_mut(&mut self) -> std::slice::IterMut<Model> {
        self.models.iter_mut()
    }

    /// Gets a mutable iterator over all enums.
    pub fn enums_mut(&mut self) -> std::slice::IterMut<Enum> {
        self.enums.iter_mut()
    }

    /// Finds a model by name.
    pub fn find_model(&self, name: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.name == name)
    }

    /// Finds a model by database name.
    pub fn find_model_db_name(&self, db_name: &str) -> Option<&Model> {
        self.models()
            .find(|model| model.database_name.as_deref() == Some(db_name))
    }

    /// Finds a model for a field reference by using reference comparison.
    pub fn find_model_by_field_ref(&self, field: &Field) -> Option<&Model> {
        // This uses the memory location of field for equality.
        self.models()
            .find(|m| m.fields().any(|f| f as *const Field == field as *const Field))
    }

    /// Finds a model for a field reference by using reference comparison.
    pub fn find_model_by_relation_field_ref(&self, field: &RelationField) -> Option<&Model> {
        // This uses the memory location of field for equality.
        self.models().find(|m| {
            m.relation_fields()
                .any(|f| f as *const RelationField == field as *const RelationField)
        })
    }

    /// Finds a field reference by a model and field name.
    pub fn find_field(&self, field: &FieldRef) -> Option<&Field> {
        // This uses the memory location of field for equality.
        self.find_model(&field.0)?.find_field(&field.1)
    }

    /// Finds a mutable scalar field reference by a model and field name.
    pub fn find_scalar_field_mut(&mut self, model: &str, field: &str) -> Option<&mut ScalarField> {
        // This uses the memory location of field for equality.
        self.find_model_mut(model)?.find_scalar_field_mut(field)
    }

    /// Finds a mutable relation field reference by a model and field name.
    pub fn find_relation_field_mut(&mut self, model: &str, field: &str) -> Option<&mut RelationField> {
        // This uses the memory location of field for equality.
        self.find_model_mut(model)?.find_relation_field_mut(field)
    }

    /// Finds an enum by name.
    pub fn find_enum(&self, name: &str) -> Option<&Enum> {
        self.enums().find(|m| m.name == *name)
    }

    /// Finds an enum by database name.
    pub fn find_enum_db_name(&self, db_name: &str) -> Option<&Enum> {
        self.enums().find(|e| e.database_name == Some(db_name.to_owned()))
    }

    /// Finds a model by name and returns a mutable reference.
    pub fn find_model_mut(&mut self, name: &str) -> Option<&mut Model> {
        self.models_mut().find(|m| m.name == *name)
    }

    /// Finds an enum by name and returns a mutable reference.
    pub fn find_enum_mut(&mut self, name: &str) -> Option<&mut Enum> {
        self.enums_mut().find(|m| m.name == *name)
    }

    /// Finds a field with a certain relation guarantee.
    /// exclude_field are necessary to avoid corner cases with self-relations (e.g. we must not recognize a field as its own related field).
    pub fn related_field(&self, from: &str, to: &str, name: &str, exclude_field: &str) -> Option<&RelationField> {
        self.find_model(&to).and_then(|related_model| {
            related_model
                .relation_fields()
                .find(|f| f.relation_info.to == from && f.relation_info.name == name && f.name != exclude_field)
        })
    }
    /// Returns (model_name, field_name) for all fields using a specific enum.
    pub fn find_enum_fields(&mut self, enum_name: &str) -> Vec<(String, String)> {
        let mut fields = vec![];

        for model in &self.models {
            for field in model.scalar_fields() {
                if FieldType::Enum(enum_name.to_owned()) == field.field_type {
                    fields.push((model.name.clone(), field.name.clone()))
                }
            }
        }
        fields
    }

    /// Returns (model_name, field_name) for all relation fields pointing to a specific model.
    pub fn find_relation_fields_for_model(&mut self, model_name: &str) -> Vec<(String, String)> {
        let mut fields = vec![];
        for model in &self.models {
            for field in model.relation_fields() {
                if field.relation_info.to == model_name {
                    fields.push((model.name.clone(), field.name.clone()))
                }
            }
        }
        fields
    }

    /// Finds a relation field related to a relation info
    pub fn find_related_field_for_info(&self, info: &RelationInfo) -> &RelationField {
        self.find_model(&info.to)
            .expect("The model referred to by a RelationInfo should always exist.")
            .relation_fields()
            .find(|f| {
                f.relation_info.name == info.name
                    && (f.relation_info.to != info.to ||
                    // This is to differentiate the opposite field from self in the self relation case.
                    (f.relation_info.to_fields != info.to_fields
                    && f.relation_info.fields != info.fields)|| f.relation_info.to != info.to)
            })
            .expect("Every RelationInfo should have a complementary RelationInfo on the opposite relation field.")
    }

    /// Returns (model_name, field_name) for all relation fields pointing to a specific model.
    pub fn find_related_info(&self, info: &RelationInfo) -> &RelationInfo {
        &self.find_related_field_for_info(info).relation_info
    }
}
