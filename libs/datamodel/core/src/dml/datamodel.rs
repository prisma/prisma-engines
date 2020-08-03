use super::*;

/// Entities in the datamodel can be flagged as `is_commented_out`. This let's the renderer
/// know that introspection encountered unsupported names or features and these are supposed
/// to be rendered as comments. Since the parser will not set these flags when reading a schema
/// string, only introspection and the lowering of the datamodel to the ast care about these flags.
/// The FieldType: Unsupported behaves in the same way.
/// Both of these are never converted into the internal datamodel.
#[derive(Debug, PartialEq, Clone)]
pub struct Datamodel {
    pub enums: Vec<Enum>,
    pub models: Vec<Model>,
}

impl Datamodel {
    pub fn new() -> Datamodel {
        Datamodel {
            enums: Vec::new(),
            models: Vec::new(),
        }
    }

    /// Checks if a datamodel contains neither enums nor models.
    pub fn is_empty(&self) -> bool {
        self.enums.is_empty() && self.models.is_empty()
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

    /// Adds a model to this datamodel.
    pub fn add_model(&mut self, model: Model) {
        self.models.push(model);
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
        self.models().find(|model| model.name == name)
    }

    /// Finds a model by database name.
    pub fn find_model_db_name(&self, db_name: &str) -> Option<&Model> {
        self.models()
            .find(|model| model.database_name.as_deref() == Some(db_name))
    }

    /// Finds parent  model for a field reference.
    pub fn find_model_by_relation_field_ref(&self, field: &RelationField) -> Option<&Model> {
        self.find_model(&self.find_related_field_bang(&field).relation_info.to)
    }

    /// Finds a mutable scalar field reference by a model and field name.
    pub fn find_scalar_field_mut(&mut self, model: &str, field: &str) -> &mut ScalarField {
        // This uses the memory location of field for equality.
        self.find_model_mut(model).find_scalar_field_mut(field)
    }

    /// Finds a mutable relation field reference by a model and field name.
    pub fn find_relation_field_mut(&mut self, model: &str, field: &str) -> &mut RelationField {
        self.find_model_mut(model).find_relation_field_mut(field)
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
    pub fn find_model_mut(&mut self, name: &str) -> &mut Model {
        self.models_mut()
            .find(|m| m.name == *name)
            .expect("We assume an internally valid datamodel before mutating.")
    }

    /// Finds an enum by name and returns a mutable reference.
    pub fn find_enum_mut(&mut self, name: &str) -> &mut Enum {
        self.enums_mut()
            .find(|m| m.name == *name)
            .expect("We assume an internally valid datamodel before mutating.")
    }

    /// Returns (model_name, field_name) for all fields using a specific enum.
    pub fn find_enum_fields(&mut self, enum_name: &str) -> Vec<(String, String)> {
        let mut fields = vec![];
        for model in self.models() {
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
        for model in self.models() {
            for field in model.relation_fields() {
                if field.relation_info.to == model_name {
                    fields.push((model.name.clone(), field.name.clone()))
                }
            }
        }
        fields
    }

    /// Finds a relation field related to a relation info
    pub fn find_related_field_for_info(&self, info: &RelationInfo, exclude: &str) -> Option<&RelationField> {
        self.find_model(&info.to)
            .expect("The model referred to by a RelationInfo should always exist.")
            .relation_fields()
            .find(|f| {
                f.relation_info.name == info.name
                    && (f.relation_info.to != info.to ||
                    // This is to differentiate the opposite field from self in the self relation case.
                    f.name != exclude)
            })
    }

    /// This finds the related field for a relationfield if available
    pub fn find_related_field(&self, rf: &RelationField) -> Option<&RelationField> {
        self.find_related_field_for_info(&rf.relation_info, &rf.name)
    }

    /// This is used once we assume the datamodel to be internally valid
    pub fn find_related_field_bang(&self, rf: &RelationField) -> &RelationField {
        self.find_related_field(rf)
            .expect("Every RelationInfo should have a complementary RelationInfo on the opposite relation field.")
    }
}
