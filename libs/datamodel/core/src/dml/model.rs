use super::*;
use crate::Field;

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
    /// Indicates if this model has to be commented out.
    pub is_commented_out: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IndexDefinition {
    pub name: Option<String>,
    pub fields: Vec<String>,
    pub tpe: IndexType,
}

impl IndexDefinition {
    pub fn is_unique(&self) -> bool {
        match self.tpe {
            IndexType::Unique => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IndexType {
    Unique,
    Normal,
}

#[derive(Debug)]
pub struct UniqueCriteria<'a> {
    pub fields: Vec<&'a ScalarField>,
}

impl<'a> UniqueCriteria<'a> {
    pub fn new(fields: Vec<&'a ScalarField>) -> UniqueCriteria<'a> {
        UniqueCriteria { fields }
    }
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
            is_commented_out: false,
        }
    }

    /// Adds a field to this model.
    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field)
    }

    /// Gets an iterator over all fields.
    pub fn fields(&self) -> std::slice::Iter<Field> {
        self.fields.iter()
    }

    /// Gets an iterator over all scalar fields.
    pub fn scalar_fields<'a>(&'a self) -> impl Iterator<Item = &'a ScalarField> + 'a {
        self.fields().filter_map(|fw| match fw {
            Field::RelationField(_) => None,
            Field::ScalarField(sf) => Some(sf),
        })
    }

    /// Gets an iterator over all relation fields.
    pub fn relation_fields<'a>(&'a self) -> impl Iterator<Item = &'a RelationField> + 'a {
        self.fields().filter_map(|fw| match fw {
            Field::RelationField(rf) => Some(rf),
            Field::ScalarField(_) => None,
        })
    }

    /// Gets a mutable iterator over all scalar fields.
    pub fn scalar_fields_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut ScalarField> + 'a {
        self.fields.iter_mut().filter_map(|fw| match fw {
            Field::RelationField(_) => None,
            Field::ScalarField(sf) => Some(sf),
        })
    }

    /// Gets a mutable iterator over all relation fields.
    pub fn relation_fields_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut RelationField> + 'a {
        self.fields.iter_mut().filter_map(|fw| match fw {
            Field::RelationField(rf) => Some(rf),
            Field::ScalarField(_) => None,
        })
    }

    /// Finds a field by name.
    pub fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields().find(|f| f.name() == name)
    }

    /// Finds a scalar field by name.
    pub fn find_scalar_field(&self, name: &str) -> Option<&ScalarField> {
        self.scalar_fields().find(|f| f.name == *name)
    }

    /// Finds a scalar field by name.
    pub fn find_relation_field(&self, name: &str) -> Option<&RelationField> {
        self.relation_fields().find(|f| f.name == *name)
    }

    /// Finds a field by database name.
    pub fn find_scalar_field_db_name(&self, db_name: &str) -> Option<&ScalarField> {
        self.scalar_fields()
            .find(|f| f.database_name.as_deref() == Some(db_name))
    }

    pub fn has_field(&self, name: &str) -> bool {
        self.find_field(name).is_some()
    }

    /// Finds a field by name and returns a mutable reference.
    pub fn find_scalar_field_mut(&mut self, name: &str) -> &mut ScalarField {
        self.scalar_fields_mut()
            .find(|f| f.name == *name)
            .expect("We assume an internally valid datamodel before mutating.")
    }

    /// Finds a relation field by name and returns a mutable reference.
    pub fn find_relation_field_mut(&mut self, name: &str) -> &mut RelationField {
        self.relation_fields_mut()
            .find(|rf| rf.name == *name)
            .expect("We assume an internally valid datamodel before mutating.")
    }

    /// Finds the name of all id fields
    pub fn id_field_names(&self) -> Vec<String> {
        let singular_id_field = self.singular_id_fields().next();
        match singular_id_field {
            Some(f) => vec![f.name.clone()],
            None => self.id_fields.clone(),
        }
    }

    /// This should match the logic in `prisma_models::Model::primary_identifier`.
    pub fn first_unique_criterion(&self) -> Vec<&ScalarField> {
        match self.strict_unique_criterias().first() {
            Some(criteria) => criteria.fields.clone(),
            None => panic!("Could not find the first unique criteria on model {}", self.name()),
        }
    }

    /// optional unique fields are NOT considered a unique criteria
    /// used for: A Model must have at least one STRICT unique criteria.
    pub fn strict_unique_criterias(&self) -> Vec<UniqueCriteria> {
        self.unique_criterias(false)
    }

    /// optional unique fields are considered a unique criteria
    /// used for: A relation must reference one LOOSE unique criteria. (optional fields are okay in this case)
    pub fn loose_unique_criterias(&self) -> Vec<UniqueCriteria> {
        self.unique_criterias(true)
    }

    /// returns the order of unique criterias ordered based on their precedence
    fn unique_criterias(&self, allow_optional: bool) -> Vec<UniqueCriteria> {
        let mut result = Vec::new();
        // first candidate: the singular id field
        {
            if let Some(x) = self.singular_id_fields().next() {
                result.push(UniqueCriteria::new(vec![x]))
            }
        }

        // second candidate: the multi field id
        {
            let id_fields: Vec<_> = self
                .id_fields
                .iter()
                .map(|f| self.find_scalar_field(&f).unwrap())
                .collect();

            if !id_fields.is_empty() {
                result.push(UniqueCriteria::new(id_fields));
            }
        }

        // third candidate: a required scalar field with a unique index.
        {
            let mut unique_required_fields: Vec<_> = self
                .scalar_fields()
                .filter(|field| field.is_unique && (field.is_required() || allow_optional))
                .map(|f| UniqueCriteria::new(vec![f]))
                .collect();

            result.append(&mut unique_required_fields);
        }

        // fourth candidate: any multi-field unique constraint where all fields are required
        {
            let mut unique_field_combi = self
                .indices
                .iter()
                .filter(|id| id.tpe == IndexType::Unique)
                .filter_map(|id| {
                    let fields: Vec<_> = id.fields.iter().map(|f| self.find_scalar_field(&f).unwrap()).collect();
                    let all_fields_are_required = fields.iter().all(|f| f.is_required());
                    if all_fields_are_required || allow_optional {
                        Some(UniqueCriteria::new(fields))
                    } else {
                        None
                    }
                })
                .collect();

            result.append(&mut unique_field_combi)
        }

        result
    }

    /// Finds the name of all id fields
    pub fn singular_id_fields(&self) -> impl std::iter::Iterator<Item = &ScalarField> {
        self.scalar_fields().filter(|x| x.is_id)
    }

    /// Determines whether there is a singular primary key
    pub fn has_single_id_field(&self) -> bool {
        self.singular_id_fields().count() == 1
    }

    /// Finds a field with a certain relation guarantee.
    /// exclude_field are necessary to avoid corner cases with self-relations (e.g. we must not recognize a field as its own related field).
    pub fn related_field(&self, to: &str, relation_name: &str, exclude_field: &str) -> Option<&RelationField> {
        self.relation_fields().find(|rf| {
            rf.relation_info.to == to
                && rf.relation_info.name == relation_name
                && (self.name != to || rf.name != exclude_field)
        })
    }

    pub fn add_index(&mut self, index: IndexDefinition) {
        self.indices.push(index)
    }

    pub fn has_created_at_and_updated_at(&self) -> bool {
        /// Finds a field by name.
        fn has_field(model: &Model, name: &str) -> bool {
            match model
                .find_scalar_field(name)
                .or_else(|| model.find_scalar_field(name.to_lowercase().as_ref()))
            {
                Some(f) => f.field_type == FieldType::Base(ScalarType::DateTime, None),
                None => false,
            }
        }

        has_field(self, "createdAt") && has_field(self, "updatedAt")
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
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }

    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}
