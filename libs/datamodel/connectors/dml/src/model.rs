use crate::field::{Field, FieldType, RelationField, ScalarField};
use crate::traits::{Ignorable, WithDatabaseName, WithName};

/// Represents a model in a prisma schema.
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
    /// Describes Primary Keys
    pub primary_key: Option<PrimaryKeyDefinition>,
    /// Indicates if this model is generated.
    pub is_generated: bool,
    /// Indicates if this model has to be commented out.
    pub is_commented_out: bool,
    /// Indicates if this model has to be ignored by the Client.
    pub is_ignored: bool,
}

/// Represents an index defined via `@@index` or `@@unique`.
#[derive(Debug, PartialEq, Clone)]
pub struct IndexDefinition {
    pub name_in_db: String,
    pub name_in_db_matches_default: bool,
    pub name_in_client: Option<String>,
    pub fields: Vec<String>,
    pub tpe: IndexType,
}

impl IndexDefinition {
    pub fn is_unique(&self) -> bool {
        matches!(self.tpe, IndexType::Unique)
    }

    pub fn is_single_field(&self) -> bool {
        self.fields.len() == 1
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IndexType {
    Unique,
    Normal,
}

/// Represents a primary key defined via `@@id` or `@id`.
#[derive(Debug, PartialEq, Clone)]
pub struct PrimaryKeyDefinition {
    pub name_in_db: Option<String>,
    pub name_in_db_matches_default: bool,
    pub name_in_client: Option<String>,
    pub fields: Vec<String>,
}

impl PrimaryKeyDefinition {
    pub fn named_with_default_name_if_necessary(&self, default_name: Option<String>) -> Self {
        let name_in_db = self.name_in_db.clone().or_else(|| default_name.clone());
        let name_in_db_matches_default = name_in_db == default_name;
        PrimaryKeyDefinition {
            name_in_db,
            name_in_db_matches_default,
            name_in_client: self.name_in_client.clone(),
            fields: self.fields.clone(),
        }
    }
}

/// A unique criteria is a set of fields through which a record can be uniquely identified.
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
            primary_key: None,
            documentation: None,
            database_name,
            is_embedded: false,
            is_generated: false,
            is_commented_out: false,
            is_ignored: false,
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

    /// Gets a mutable  iterator over all fields.
    pub fn fields_mut(&mut self) -> impl Iterator<Item = &mut Field> {
        self.fields.iter_mut()
    }

    /// Gets an iterator over all scalar fields.
    pub fn scalar_fields(&self) -> impl Iterator<Item = &ScalarField> {
        self.fields.iter().filter_map(|f| f.as_scalar_field())
    }

    /// Gets an iterator over all relation fields.
    pub fn relation_fields(&self) -> impl Iterator<Item = &RelationField> {
        self.fields.iter().filter_map(|f| f.as_relation_field())
    }

    /// Gets a mutable iterator over all scalar fields.
    pub fn scalar_fields_mut(&mut self) -> impl Iterator<Item = &mut ScalarField> {
        self.fields_mut().filter_map(|fw| match fw {
            Field::RelationField(_) => None,
            Field::ScalarField(sf) => Some(sf),
        })
    }

    /// Gets a mutable iterator over all relation fields.
    pub fn relation_fields_mut(&mut self) -> impl Iterator<Item = &mut RelationField> {
        self.fields_mut().filter_map(|fw| match fw {
            Field::RelationField(rf) => Some(rf),
            Field::ScalarField(_) => None,
        })
    }

    /// Finds a field by name.
    pub fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields().find(|f| f.name() == name)
    }

    /// Finds a field by name and returns a mutable reference.
    pub fn find_field_mut(&mut self, name: &str) -> &mut Field {
        self.fields_mut().find(|f| f.name() == name).unwrap()
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
        let model_name = &self.name.clone();
        self.scalar_fields_mut().find(|rf| rf.name == *name).expect(&*format!(
            "Could not find scalar field {} on model {}.",
            name, model_name
        ))
    }

    /// Finds a relation field by name and returns a mutable reference.
    pub fn find_relation_field_mut(&mut self, name: &str) -> &mut RelationField {
        let model_name = &self.name.clone();
        self.relation_fields_mut().find(|rf| rf.name == *name).expect(&*format!(
            "Could not find relation field {} on model {}.",
            name, model_name
        ))
    }

    /// Finds the name of all id fields
    pub fn id_field_names(&self) -> Vec<String> {
        self.primary_key.as_ref().map_or(vec![], |pk| pk.fields.clone())
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
        self.unique_criterias(false, false)
    }

    /// optional unique fields are NOT considered a unique criteria
    /// used for: A Model must have at least one STRICT unique criteria.
    /// Ignores unsupported, used for introspection to decide when to ignore
    pub fn strict_unique_criterias_disregarding_unsupported(&self) -> Vec<UniqueCriteria> {
        self.unique_criterias(false, true)
    }

    /// optional unique fields are considered a unique criteria
    /// used for: A relation must reference one LOOSE unique criteria. (optional fields are okay in this case)
    pub fn loose_unique_criterias(&self) -> Vec<UniqueCriteria> {
        self.unique_criterias(true, false)
    }

    /// returns the order of unique criterias ordered based on their precedence
    fn unique_criterias(&self, allow_optional: bool, disregard_unsupported: bool) -> Vec<UniqueCriteria> {
        let mut result = Vec::new();

        let ineligible = |field: &ScalarField| {
            if disregard_unsupported {
                field.is_commented_out || matches!(field.field_type, FieldType::Unsupported(_))
            } else {
                field.is_commented_out
            }
        };

        // first candidate: the primary key
        {
            let id_fields: Vec<_> = self.primary_key.as_ref().map_or(vec![], |pk| {
                pk.fields.iter().map(|f| self.find_scalar_field(&f).unwrap()).collect()
            });

            if !id_fields.is_empty()
                && !id_fields
                    .iter()
                    .any(|f| ineligible(f) || (f.is_optional() && !allow_optional))
            {
                result.push(UniqueCriteria::new(id_fields));
            }
        }

        // second candidate: any unique constraint where all fields are required
        {
            let mut unique_field_combi: Vec<UniqueCriteria> = self
                .indices
                .iter()
                .filter(|id| id.tpe == IndexType::Unique)
                .filter_map(|id| {
                    let fields: Vec<_> = id.fields.iter().map(|f| self.find_scalar_field(&f).unwrap()).collect();
                    let no_fields_are_ineligible = !fields.iter().any(|f| ineligible(f));
                    let all_fields_are_required = fields.iter().all(|f| f.is_required());
                    ((all_fields_are_required || allow_optional) && no_fields_are_ineligible)
                        .then(|| UniqueCriteria::new(fields))
                })
                .collect();

            unique_field_combi.sort_by_key(|c| c.fields.len());

            result.extend(unique_field_combi)
        }

        result
    }

    pub fn field_is_indexed(&self, field_name: &str) -> bool {
        if let Some(field) = self.find_field(field_name) {
            if field.is_id() || field.is_unique() {
                return true;
            }

            let is_first_in_index = self
                .indices
                .iter()
                .any(|index| index.fields.first().unwrap() == field_name);

            let is_first_in_primary_key =
                matches!(&self.primary_key, Some(pk) if pk.fields.first().unwrap() == field_name);

            return is_first_in_index || is_first_in_primary_key;
        }

        false
    }

    /// Finds the name of all id fields
    pub fn singular_id_fields(&self) -> impl std::iter::Iterator<Item = &ScalarField> {
        self.scalar_fields().filter(|x| x.is_id())
    }

    /// Finds all fields defined as autoincrement
    pub fn auto_increment_fields(&self) -> impl std::iter::Iterator<Item = &ScalarField> {
        self.scalar_fields().filter(|x| x.is_auto_increment())
    }

    /// Determines whether there is a singular primary key
    pub fn has_singular_id(&self) -> bool {
        matches!(&self.primary_key, Some(pk) if pk.fields.len() == 1)
    }

    /// Determines whether there is a singular primary key
    pub fn has_compound_id(&self) -> bool {
        matches!(&self.primary_key, Some(pk) if pk.fields.len() > 1)
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
                Some(f) => f.field_type.is_datetime(),
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

impl Ignorable for Model {
    fn is_ignored(&self) -> bool {
        self.is_ignored
    }

    fn ignore(&mut self) {
        self.is_ignored = true;
    }
}
