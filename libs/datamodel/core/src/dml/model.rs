use super::*;

/// Represents a model in a prisma datamodel.
#[derive(Debug, PartialEq, Clone)]
pub struct Model {
    /// Name of the model.
    pub name: String,
    /// Fields of the model.
    pub fields: Vec<Field>,
    /// Comments associated with this model.
    pub documentation: Option<String>,
    /// The database internal name of this model.
    pub database_name: Option<String>,
    /// Indicates if this model is embedded or not.
    pub is_embedded: bool,
    /// Describes Composite Indexes
    pub indices: Vec<IndexDefinition>,
    /// Describes Composite Primary Keys
    pub id_fields: Vec<String>,
    /// Indicates if this model is generated.
    pub is_generated: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IndexDefinition {
    pub name: Option<String>,
    pub fields: Vec<String>,
    pub tpe: IndexType,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IndexType {
    Unique,
    Normal,
}

impl Model {
    /// Creates a new model with the given name.
    pub fn new(name: String, database_name: Option<String>) -> Model {
        Model {
            name,
            fields: vec![],
            indices: vec![],
            id_fields: vec![],
            documentation: None,
            database_name,
            is_embedded: false,
            is_generated: false,
        }
    }

    /// Adds a field to this model.
    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field)
    }

    /// Removes a field with the given name from this model.
    pub fn remove_field(&mut self, name: &str) {
        self.fields.retain(|f| f.name != name);
    }

    /// Gets an iterator over all fields.
    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    /// Gets a mutable iterator over all fields.
    pub fn fields_mut(&mut self) -> std::slice::IterMut<Field> {
        self.fields.iter_mut()
    }

    /// Finds a field by name.
    pub fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields().find(|f| f.name == *name)
    }

    /// Finds a field by name and returns a mutable reference.
    pub fn find_field_mut(&mut self, name: &str) -> Option<&mut Field> {
        self.fields_mut().find(|f| f.name == *name)
    }

    /// Finds the name of all id fields
    pub fn id_field_names(&self) -> Vec<String> {
        let singular_id_field = self.singular_id_fields().next();
        match singular_id_field {
            Some(f) => vec![f.name.clone()],
            None => self.id_fields.clone(),
        }
    }

    /// Finds the name of all id fields
    pub fn singular_id_fields(&self) -> impl std::iter::Iterator<Item = &Field> {
        self.fields().filter(|x| x.id_info.is_some())
    }

    /// Finds a field with a certain relation guarantee.
    /// exclude_field are necessary to avoid corner cases with self-relations (e.g. we must not recognize a field as its own related field).
    pub fn related_field(&self, to: &str, relation_name: &str, exclude_field: &str) -> Option<&Field> {
        self.fields().find(|f| {
            if let FieldType::Relation(rel_info) = &f.field_type {
                if rel_info.to == to && rel_info.name == relation_name && (self.name != to || f.name != exclude_field) {
                    return true;
                }
            }
            false
        })
    }

    /// Finds a mutable field with a certain relation guarantee.
    pub fn related_field_mut(&mut self, to: &str, name: &str, exclude_field: &str) -> Option<&mut Field> {
        let self_name = self.name.clone();
        self.fields_mut().find(|f| {
            if let FieldType::Relation(rel_info) = &f.field_type {
                if rel_info.to == to && rel_info.name == name && (self_name != to || f.name != exclude_field) {
                    return true;
                }
            }

            false
        })
    }

    /// Checks if this is a relation model. A relation model has exactly
    /// two relations, which are required.
    pub fn is_relation_model(&self) -> bool {
        let related_fields = self.fields().filter(|f| -> bool {
            if let FieldType::Relation(_) = f.field_type {
                f.arity == FieldArity::Required
            } else {
                false
            }
        });

        related_fields.count() == 2
    }

    /// Checks if this is a pure relation model.
    /// It has only two fields, both of them are required relations.
    pub fn is_pure_relation_model(&self) -> bool {
        self.is_relation_model() && self.fields.len() == 2
    }

    pub fn add_index(&mut self, index: IndexDefinition) {
        self.indices.push(index)
    }

    pub fn has_index(&self, index: &IndexDefinition) -> bool {
        self.indices.iter().any(|own_index| own_index == index)
    }
}

impl WithName for Model {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for Model {
    fn database_names(&self) -> Vec<&str> {
        match &self.database_name {
            None => vec![],
            Some(db_name) => vec![db_name],
        }
    }

    fn set_database_names(&mut self, database_names: Vec<String>) -> Result<(), String> {
        if database_names.len() > 1 {
            Err("A Model must not specify multiple mapped names.".to_string())
        } else {
            let first = database_names.into_iter().next();
            self.database_name = first;
            Ok(())
        }
    }
}
