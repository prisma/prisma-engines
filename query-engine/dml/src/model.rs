use crate::field::{Field, ScalarField};
use crate::traits::{Ignorable, WithDatabaseName, WithName};
use indoc::formatdoc;
use psl_core::parser_database::{ast, IndexType};
use std::{borrow::Cow, fmt};

/// Represents a model in a prisma schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub id: ast::ModelId,
    /// Name of the model.
    pub name: String,
    /// Fields of the model.
    pub fields: Vec<Field>,
    /// Comments associated with this model.
    pub documentation: Option<String>,
    /// The database internal name of this model.
    pub database_name: Option<String>,
    /// Describes Composite Indexes
    pub indices: Vec<IndexDefinition>,
    /// Describes the Primary Keys
    pub primary_key: Option<PrimaryKeyDefinition>,
    /// Indicates if this model has to be ignored by the Client.
    pub is_ignored: bool,
    /// The contents of the `@@schema("...")` attribute.
    pub schema: Option<String>,
    /// If the model is defined as a view in the database.
    pub is_view: bool,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IndexAlgorithm {
    BTree,
    Hash,
    Gist,
    Gin,
    SpGist,
    Brin,
}

impl Default for IndexAlgorithm {
    fn default() -> Self {
        Self::BTree
    }
}

impl AsRef<str> for IndexAlgorithm {
    fn as_ref(&self) -> &str {
        match self {
            IndexAlgorithm::BTree => "BTree",
            IndexAlgorithm::Hash => "Hash",
            IndexAlgorithm::Gist => "Gist",
            IndexAlgorithm::Gin => "Gin",
            IndexAlgorithm::SpGist => "SpGist",
            IndexAlgorithm::Brin => "Brin",
        }
    }
}

impl fmt::Display for IndexAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

/// Represents an index defined via `@@index`, `@unique` or `@@unique`.
#[derive(Debug, PartialEq, Clone)]
pub struct IndexDefinition {
    pub name: Option<String>,
    pub db_name: Option<String>,
    pub fields: Vec<IndexField>,
    pub tpe: IndexType,
    pub clustered: Option<bool>,
    pub algorithm: Option<IndexAlgorithm>,
    pub defined_on_field: bool,
}

impl IndexDefinition {
    pub fn is_unique(&self) -> bool {
        matches!(self.tpe, IndexType::Unique)
    }

    pub fn is_fulltext(&self) -> bool {
        matches!(self.tpe, IndexType::Fulltext)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum OperatorClass {
    // GiST
    InetOps,

    // GIN
    JsonbOps,
    JsonbPathOps,
    ArrayOps,

    // SP-GiST
    TextOps,

    // BRIN
    BitMinMaxOps,
    VarBitMinMaxOps,
    BpcharBloomOps,
    BpcharMinMaxOps,
    ByteaBloomOps,
    ByteaMinMaxOps,
    DateBloomOps,
    DateMinMaxOps,
    DateMinMaxMultiOps,
    Float4BloomOps,
    Float4MinMaxOps,
    Float4MinMaxMultiOps,
    Float8BloomOps,
    Float8MinMaxOps,
    Float8MinMaxMultiOps,
    InetInclusionOps,
    InetBloomOps,
    InetMinMaxOps,
    InetMinMaxMultiOps,
    Int2BloomOps,
    Int2MinMaxOps,
    Int2MinMaxMultiOps,
    Int4BloomOps,
    Int4MinMaxOps,
    Int4MinMaxMultiOps,
    Int8BloomOps,
    Int8MinMaxOps,
    Int8MinMaxMultiOps,
    NumericBloomOps,
    NumericMinMaxOps,
    NumericMinMaxMultiOps,
    OidBloomOps,
    OidMinMaxOps,
    OidMinMaxMultiOps,
    TextBloomOps,
    TextMinMaxOps,
    TimestampBloomOps,
    TimestampMinMaxOps,
    TimestampMinMaxMultiOps,
    TimestampTzBloomOps,
    TimestampTzMinMaxOps,
    TimestampTzMinMaxMultiOps,
    TimeBloomOps,
    TimeMinMaxOps,
    TimeMinMaxMultiOps,
    TimeTzBloomOps,
    TimeTzMinMaxOps,
    TimeTzMinMaxMultiOps,
    UuidBloomOps,
    UuidMinMaxOps,
    UuidMinMaxMultiOps,

    Raw(Cow<'static, str>),
}

impl AsRef<str> for OperatorClass {
    fn as_ref(&self) -> &str {
        match self {
            Self::InetOps => "InetOps",
            Self::JsonbOps => "JsonbOps",
            Self::JsonbPathOps => "JsonbPathOps",
            Self::ArrayOps => "ArrayOps",
            Self::TextOps => "TextOps",
            Self::BitMinMaxOps => "BitMinMaxOps",
            Self::VarBitMinMaxOps => "VarBitMinMaxOps",
            Self::BpcharBloomOps => "BpcharBloomOps",
            Self::BpcharMinMaxOps => "BpcharMinMaxOps",
            Self::ByteaBloomOps => "ByteaBloomOps",
            Self::ByteaMinMaxOps => "ByteaMinMaxOps",
            Self::DateBloomOps => "DateBloomOps",
            Self::DateMinMaxOps => "DateMinMaxOps",
            Self::DateMinMaxMultiOps => "DateMinMaxMultiOps",
            Self::Float4BloomOps => "Float4BloomOps",
            Self::Float4MinMaxOps => "Float4MinMaxOps",
            Self::Float4MinMaxMultiOps => "Float4MinMaxMultiOps",
            Self::Float8BloomOps => "Float8BloomOps",
            Self::Float8MinMaxOps => "Float8MinMaxOps",
            Self::Float8MinMaxMultiOps => "Float8MinMaxMultiOps",
            Self::InetInclusionOps => "InetInclusionOps",
            Self::InetBloomOps => "InetBloomOps",
            Self::InetMinMaxOps => "InetMinMaxOps",
            Self::InetMinMaxMultiOps => "InetMinMaxMultiOps",
            Self::Int2BloomOps => "Int2BloomOps",
            Self::Int2MinMaxOps => "Int2MinMaxOps",
            Self::Int2MinMaxMultiOps => "Int2MinMaxMultiOps",
            Self::Int4BloomOps => "Int4BloomOps",
            Self::Int4MinMaxOps => "Int4MinMaxOps",
            Self::Int4MinMaxMultiOps => "Int4MinMaxMultiOps",
            Self::Int8BloomOps => "Int8BloomOps",
            Self::Int8MinMaxOps => "Int8MinMaxOps",
            Self::Int8MinMaxMultiOps => "Int8MinMaxMultiOps",
            Self::NumericBloomOps => "NumericBloomOps",
            Self::NumericMinMaxOps => "NumericMinMaxOps",
            Self::NumericMinMaxMultiOps => "NumericMinMaxMultiOps",
            Self::OidBloomOps => "OidBloomOps",
            Self::OidMinMaxOps => "OidMinMaxOps",
            Self::OidMinMaxMultiOps => "OidMinMaxMultiOps",
            Self::TextBloomOps => "TextBloomOps",
            Self::TextMinMaxOps => "TextMinMaxOps",
            Self::TimestampBloomOps => "TimestampBloomOps",
            Self::TimestampMinMaxOps => "TimestampMinMaxOps",
            Self::TimestampMinMaxMultiOps => "TimestampMinMaxMultiOps",
            Self::TimestampTzBloomOps => "TimestampTzBloomOps",
            Self::TimestampTzMinMaxOps => "TimestampTzMinMaxOps",
            Self::TimestampTzMinMaxMultiOps => "TimestampTzMinMaxMultiOps",
            Self::TimeBloomOps => "TimeBloomOps",
            Self::TimeMinMaxOps => "TimeMinMaxOps",
            Self::TimeMinMaxMultiOps => "TimeMinMaxMultiOps",
            Self::TimeTzBloomOps => "TimeTzBloomOps",
            Self::TimeTzMinMaxOps => "TimeTzMinMaxOps",
            Self::TimeTzMinMaxMultiOps => "TimeTzMinMaxMultiOps",
            Self::UuidBloomOps => "UuidBloomOps",
            Self::UuidMinMaxOps => "UuidMinMaxOps",
            Self::UuidMinMaxMultiOps => "UuidMinMaxOps",
            Self::Raw(s) => s.as_ref(),
        }
    }
}

impl fmt::Display for OperatorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl OperatorClass {
    pub fn raw(op: impl Into<Cow<'static, str>>) -> Self {
        Self::Raw(op.into())
    }

    pub fn as_raw(&self) -> Option<&str> {
        match self {
            Self::Raw(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, Self::Raw(_))
    }
}

/// A field in an index that optionally defines a sort order and length limit.
#[derive(Debug, PartialEq, Clone)]
pub struct IndexField {
    pub path: Vec<(String, Option<String>)>,
    pub sort_order: Option<SortOrder>,
    pub length: Option<u32>,
    pub operator_class: Option<OperatorClass>,
}

impl IndexField {
    /// Tests only
    pub fn new_in_model(name: &str) -> Self {
        IndexField {
            path: vec![(name.into(), None)],
            sort_order: None,
            length: None,
            operator_class: None,
        }
    }

    pub fn new_in_path(path: &[(&str, Option<&str>)]) -> Self {
        IndexField {
            path: path
                .iter()
                .map(|(k, v)| (k.to_string(), v.map(|v| v.to_string())))
                .collect(),
            sort_order: None,
            length: None,
            operator_class: None,
        }
    }

    pub fn from_field(&self) -> &str {
        &self.path.first().unwrap().0
    }
}

/// Represents a primary key defined via `@@id` or `@id`.
#[derive(Debug, PartialEq, Clone)]
pub struct PrimaryKeyDefinition {
    pub name: Option<String>,
    pub db_name: Option<String>,
    pub fields: Vec<PrimaryKeyField>,
}

/// A field in a Primary Key that optionally defines a sort order and length limit.
#[derive(Debug, PartialEq, Clone)]
pub struct PrimaryKeyField {
    pub name: String,
    pub sort_order: Option<SortOrder>,
    pub length: Option<u32>,
}

impl PrimaryKeyField {
    /// Tests only
    pub fn new(name: &str) -> Self {
        PrimaryKeyField {
            name: name.to_string(),
            sort_order: None,
            length: None,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl AsRef<str> for SortOrder {
    fn as_ref(&self) -> &str {
        match self {
            SortOrder::Asc => "Asc",
            SortOrder::Desc => "Desc",
        }
    }
}

/// A unique criterion is a set of fields through which a record can be uniquely identified.
#[derive(Debug)]
pub struct UniqueCriterion<'a> {
    pub fields: Vec<&'a ScalarField>,
}

impl<'a> UniqueCriterion<'a> {
    pub fn new(fields: Vec<&'a ScalarField>) -> UniqueCriterion<'a> {
        UniqueCriterion { fields }
    }
}

impl Model {
    /// Creates a new model with the given name.
    pub fn new(id: ast::ModelId, name: String, database_name: Option<String>) -> Model {
        Model {
            id,
            name,
            fields: vec![],
            indices: vec![],
            primary_key: None,
            documentation: None,
            database_name,
            is_ignored: false,
            is_view: false,
            schema: None,
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
    pub fn scalar_fields(&self) -> impl Iterator<Item = &ScalarField> {
        self.fields.iter()
    }

    /// Finds a field by name.
    pub fn find_field(&self, name: &str) -> Option<&Field> {
        self.fields().find(|f| f.name() == name)
    }

    /// Finds a scalar field by name.
    pub fn find_scalar_field(&self, name: &str) -> Option<&ScalarField> {
        self.scalar_fields().find(|f| f.name == *name)
    }

    /// This should match the logic in `prisma_models::Model::primary_identifier`.
    pub fn first_unique_criterion(&self) -> Vec<&ScalarField> {
        match self.strict_unique_criterias().first() {
            Some(criteria) => criteria.fields.clone(),
            None => panic!("Could not find the first unique criteria on model {}", self.name()),
        }
    }

    /// Optional unique fields are NOT considered a unique criterion.
    ///
    /// Used for: A Model must have at least one STRICT unique criteria.
    pub fn strict_unique_criterias(&self) -> Vec<UniqueCriterion> {
        self.unique_criterias()
    }

    /// Returns the order of unique criterias ordered based on their precedence
    fn unique_criterias(&self) -> Vec<UniqueCriterion> {
        let mut result = Vec::new();

        // first candidate: primary key
        {
            if let Some(pk) = &self.primary_key {
                let id_fields: Vec<_> = pk
                    .fields
                    .iter()
                    .map(|f| match self.find_scalar_field(&f.name) {
                        Some(field) => field,
                        None => {
                            let error = formatdoc!(
                                r#"
                                Hi there! We've been seeing this error in our error reporting backend,
                                but cannot reproduce it in our own tests. The problem is that we have a
                                primary key in the model `{}` that uses the column `{}` which we for
                                some reason don't have in our internal representation. If you see this,
                                could you please file an issue to https://github.com/prisma/prisma so we
                                can discuss about fixing this. -- Your friendly prisma developers.
                            "#,
                                self.name,
                                f.name
                            );

                            panic!("{}", error.replace('\n', " "));
                        }
                    })
                    .collect();

                if !id_fields.is_empty() && !id_fields.iter().any(|f| f.is_optional()) {
                    result.push(UniqueCriterion::new(id_fields));
                }
            }
        }

        // second candidate: any unique constraint where all fields are required
        {
            let mut unique_field_combi: Vec<UniqueCriterion> = self
                .indices
                .iter()
                .filter(|id| id.is_unique())
                .filter_map(|id| {
                    let fields: Vec<_> = id
                        .fields
                        .iter()
                        // TODO: remove this when supporting composite indices on QE
                        .filter(|f| f.path.len() == 1)
                        .map(|f| &f.path.first().unwrap().0)
                        .map(|name| self.find_scalar_field(name).unwrap())
                        .collect();
                    let all_fields_are_required = fields.iter().all(|f| f.is_required());
                    all_fields_are_required.then(|| UniqueCriterion::new(fields))
                })
                .collect();

            unique_field_combi.sort_by_key(|c| c.fields.len());

            result.extend(unique_field_combi)
        }

        result
    }

    pub fn field_is_indexed(&self, name: &str) -> bool {
        let field = self.find_field(name).unwrap();

        if self.field_is_primary(field.name()) || self.field_is_unique(field.name()) {
            return true;
        }

        let is_first_in_index = self.indices.iter().any(|index| {
            index
                .fields
                .iter()
                .flat_map(|f| &f.path)
                .last()
                .map(|(field_name, _)| field_name == name)
                .unwrap_or(false)
        });

        let is_first_in_primary_key = matches!(&self.primary_key, Some(PrimaryKeyDefinition{ fields, ..}) if fields.first().unwrap().name == name);

        is_first_in_index || is_first_in_primary_key
    }

    /// Determines whether there is a singular primary key
    pub fn has_single_id_field(&self) -> bool {
        matches!(&self.primary_key, Some(PrimaryKeyDefinition{fields, ..}) if fields.len() ==1)
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

    pub fn field_is_unique(&self, name: &str) -> bool {
        self.indices.iter().any(|i| {
            let names_match = i
                .fields
                .iter()
                .flat_map(|f| &f.path)
                .last()
                .map(|(field_name, _)| field_name == name)
                .unwrap_or(false);

            i.is_unique() && i.fields.len() == 1 && names_match
        })
    }

    pub fn field_is_unique_and_defined_on_field(&self, name: &str) -> bool {
        self.indices.iter().any(|i| {
            let names_match = i
                .fields
                .iter()
                .flat_map(|f| &f.path)
                .last()
                .map(|(field_name, _)| field_name == name)
                .unwrap_or(false);

            i.is_unique() && i.fields.len() == 1 && names_match && i.defined_on_field
        })
    }

    pub fn field_is_primary(&self, field_name: &str) -> bool {
        matches!(&self.primary_key, Some(pk) if pk.fields.len() == 1 && pk.fields.first().unwrap().name == field_name)
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
