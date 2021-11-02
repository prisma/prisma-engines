mod name;

pub(crate) use name::Name;

use super::{field_type::FieldType, CompositeTypeDepth};
use bson::{Bson, Document};
use convert_case::{Case, Casing};
use datamodel::{
    CompositeType, CompositeTypeField, Datamodel, DefaultValue, Field, IndexDefinition, IndexType, Model,
    NativeTypeInstance, PrimaryKeyDefinition, ScalarField, ScalarType, ValueGenerator, WithDatabaseName,
};
use introspection_connector::Warning;
use mongodb::IndexModel;
use native_types::MongoDbType;
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fmt,
    ops::Deref,
};

pub(super) const SAMPLE_SIZE: i32 = 1000;

static RESERVED_NAMES: &[&str] = &["PrismaClient"];

/// Statistical data from a MongoDB database for determining a Prisma data
/// model.
#[derive(Debug, Default)]
pub(super) struct Statistics {
    /// (model_name, field_name) -> type percentages
    fields: BTreeMap<(Name, String), FieldSampler>,
    /// model_name -> document count
    models: HashMap<Name, usize>,
    /// model_name -> indices
    indices: BTreeMap<String, Vec<IndexModel>>,
    /// How deep we travel in nested composite types until switching to Json. None will always use
    /// Json, Some(-1) will never switch to Json.
    composite_type_depth: CompositeTypeDepth,
}

impl Statistics {
    pub(super) fn new(composite_type_depth: CompositeTypeDepth) -> Self {
        Self {
            composite_type_depth,
            ..Default::default()
        }
    }

    /// Track a collection as prisma model.
    pub(super) fn track_model(&mut self, model: &str) {
        self.models.entry(Name::Model(model.to_string())).or_insert(0);
    }

    pub(super) fn track_model_fields(&mut self, model: &str, document: Document) {
        self.track_document_types(Name::Model(model.to_string()), &document, self.composite_type_depth);
    }

    /// Track an index for the given model.
    pub(super) fn track_index(&mut self, model_name: &str, index: IndexModel) {
        let indexes = self.indices.entry(model_name.to_string()).or_default();
        indexes.push(index);
    }

    /// From the given data, create a Prisma data model with best effort basis.
    pub(super) fn into_datamodel(self, warnings: &mut Vec<Warning>) -> Datamodel {
        let mut data_model = Datamodel::new();
        let mut indices = self.indices;
        let mut unsupported = Vec::new();
        let mut undecided_types = Vec::new();

        let mut models: BTreeMap<String, Model> = self
            .models
            .iter()
            .flat_map(|(name, _)| name.as_model_name())
            .map(|model_name| (model_name.to_string(), new_model(model_name)))
            .collect();

        let mut types: BTreeMap<String, CompositeType> = self
            .models
            .iter()
            .flat_map(|(name, _)| name.as_type_name())
            .map(|type_name| (type_name.to_string(), new_composite_type(type_name)))
            .collect();

        for ((name, field_name), sampler) in self.fields.into_iter() {
            let doc_count = *self.models.get(&name).unwrap_or(&0);
            let field_count = sampler.counter;

            let percentages = sampler.percentages();

            let field_type = match percentages.find_most_common() {
                Some(field_type) => field_type.to_owned(),
                None => FieldType::Unsupported("Unknown"),
            };

            if let FieldType::Unsupported(r#type) = field_type {
                unsupported.push((name.to_string(), field_name.to_string(), r#type));
            }

            if percentages.data.len() > 1 {
                undecided_types.push((name.to_string(), field_name.to_string(), field_type.to_string()));
            }

            let arity = if field_type.is_array() {
                datamodel::FieldArity::List
            } else if doc_count > field_count || sampler.nullable {
                datamodel::FieldArity::Optional
            } else {
                datamodel::FieldArity::Required
            };

            let documentation = if percentages.has_type_variety() {
                Some(format!(
                    "Multiple data types found: {} out of {} sampled entries",
                    percentages, field_count
                ))
            } else {
                None
            };

            let (sanitized_name, database_name) = match sanitize_string(&field_name) {
                Some(sanitized) => (sanitized, Some(field_name)),
                None if field_name == "id" => ("id_".to_string(), Some(field_name)),
                None => (field_name, None),
            };

            match name {
                Name::Model(model_name) => {
                    let model = models.get_mut(&model_name).unwrap();

                    if database_name.as_deref() == Some("_id") {
                        continue;
                    }

                    model.fields.push(Field::ScalarField(ScalarField {
                        name: sanitized_name,
                        field_type: field_type.into(),
                        arity,
                        database_name,
                        default_value: None,
                        documentation,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }));
                }
                Name::CompositeType(type_name) => {
                    let r#type = types.get_mut(&type_name).unwrap();

                    r#type.fields.push(CompositeTypeField {
                        name: sanitized_name,
                        r#type: field_type.into(),
                        arity,
                        documentation,
                        database_name,
                    });
                }
            }
        }

        add_indices_to_models(&mut models, &mut indices);

        for (_, model) in models.into_iter() {
            data_model.add_model(model);
        }

        for (_, composite_type) in types.into_iter() {
            data_model.composite_types.push(composite_type);
        }

        if !unsupported.is_empty() {
            warnings.push(crate::warnings::unsupported_type(&unsupported));
        }

        if !undecided_types.is_empty() {
            warnings.push(crate::warnings::undecided_field_type(&undecided_types));
        }

        data_model
    }

    fn composite_type_name(&self, model: &str, field: &str) -> Name {
        let name = Name::Model(format!("{}_{}", model, field).to_case(Case::Pascal));

        let name = if self.models.contains_key(&name) {
            format!("{}_", name)
        } else {
            name.take()
        };

        Name::CompositeType(name)
    }

    fn track_composite_type_fields(
        &mut self,
        model: &str,
        field: &str,
        document: &Document,
        depth: CompositeTypeDepth,
    ) {
        let name = self.composite_type_name(model, field);
        self.track_document_types(name, document, depth);
    }

    fn find_and_track_composite_types(
        &mut self,
        model: &str,
        field: &str,
        bson: &Bson,
        depth: CompositeTypeDepth,
    ) -> (usize, bool) {
        let mut array_depth = 0;
        let mut found = false;

        let mut documents = vec![bson];

        while let Some(bson) = documents.pop() {
            match bson {
                Bson::Document(doc) => {
                    found = true;

                    if array_depth < 2 {
                        self.track_composite_type_fields(model, field, doc, depth);
                    }
                }
                Bson::Array(ary) => {
                    array_depth += 1;

                    for bson in ary.iter() {
                        documents.push(bson);
                    }
                }
                _ => (),
            }
        }

        (array_depth, found)
    }

    /// Track all fields and field types from the given document.
    fn track_document_types(&mut self, name: Name, document: &Document, depth: CompositeTypeDepth) {
        if name.is_composite_type() && depth.is_none() {
            return;
        }

        let doc_count = self.models.entry(name.clone()).or_default();
        *doc_count += 1;

        let depth = match name {
            Name::CompositeType(_) => depth.level_down(),
            _ => depth,
        };

        for (field, val) in document.into_iter() {
            let (array_layers, found_composite) = self.find_and_track_composite_types(name.as_ref(), field, val, depth);

            let compound_name = if found_composite && !depth.is_none() {
                Some(self.composite_type_name(name.as_ref(), field))
            } else {
                None
            };

            let sampler = self.fields.entry((name.clone(), field.to_string())).or_default();
            sampler.counter += 1;

            match FieldType::from_bson(val, compound_name) {
                Some(_) if found_composite && array_layers > 1 => {
                    let counter = sampler
                        .types
                        .entry(FieldType::Array(Box::new(FieldType::Json)))
                        .or_default();
                    *counter += 1;
                }
                Some(field_type) => {
                    let counter = sampler.types.entry(field_type).or_default();
                    *counter += 1;
                }
                None => {
                    sampler.nullable = true;
                }
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct FieldSampler {
    types: BTreeMap<FieldType, usize>,
    nullable: bool,
    counter: usize,
}

impl FieldSampler {
    fn percentages(&self) -> FieldPercentages {
        let total = self.types.iter().fold(0, |acc, (_, count)| acc + count);
        let mut data = BTreeMap::new();

        for (typ, count) in self.types.iter() {
            let p = data.entry(typ.clone()).or_default();
            *p = (*count as f64) / (total as f64);
        }

        FieldPercentages { data }
    }
}

#[derive(Debug)]
struct FieldPercentages {
    data: BTreeMap<FieldType, f64>,
}

impl fmt::Display for FieldPercentages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, (k, v)) in self.data.iter().enumerate() {
            let p = (*v * 1000.0).round() / 10.0;
            write!(f, "{}: {}%", k, p)?;

            if i < self.data.keys().count() - 1 {
                write!(f, ", ")?;
            }
        }

        Ok(())
    }
}

impl FieldPercentages {
    /// The most prominent choice for the field type, based on the tracked data.
    fn find_most_common(&self) -> Option<FieldType> {
        self.data
            .iter()
            .max_by(|left, right| left.1.partial_cmp(right.1).unwrap_or(Ordering::Equal))
            .map(|(r#type, _)| r#type.clone())
    }

    /// Dirty data...
    fn has_type_variety(&self) -> bool {
        self.data.len() > 1
    }
}

fn new_composite_type(type_name: &str) -> CompositeType {
    CompositeType {
        name: type_name.to_string(),
        fields: Vec::new(),
    }
}

fn new_model(model_name: &str) -> Model {
    let primary_key = PrimaryKeyDefinition {
        name: None,
        db_name: None,
        fields: vec!["id".to_string()],
        defined_on_field: true,
    };

    let field_type = datamodel::FieldType::Scalar(
        ScalarType::String,
        None,
        Some(NativeTypeInstance::new("ObjectId", Vec::new(), &MongoDbType::ObjectId)),
    );

    let primary_key_field = Field::ScalarField({
        let mut sf = ScalarField::new("id", datamodel::FieldArity::Required, field_type);

        sf.set_database_name(Some("_id".to_string()));
        sf.set_default_value(DefaultValue::new_expression(
            ValueGenerator::new("dbgenerated".to_owned(), Vec::new()).unwrap(),
        ));

        sf
    });

    let (name, database_name, documentation) = match sanitize_string(model_name) {
        Some(sanitized) => (Cow::from(sanitized), Some(model_name.to_string()), None),
        None if RESERVED_NAMES.contains(&model_name) => {
            let documentation = "This model has been renamed to 'RenamedPrismaClient' during introspection, because the original name 'PrismaClient' is reserved.";

            (
                Cow::from(format!("Renamed{}", model_name)),
                Some(model_name.to_string()),
                Some(documentation.to_string()),
            )
        }
        None => (Cow::from(model_name), None, None),
    };

    Model {
        name: name.to_string(),
        primary_key: Some(primary_key),
        fields: vec![primary_key_field],
        database_name,
        documentation,
        ..Default::default()
    }
}

fn add_indices_to_models(models: &mut BTreeMap<String, Model>, indices: &mut BTreeMap<String, Vec<IndexModel>>) {
    for (model_name, model) in models.iter_mut() {
        for index in indices.remove(model_name).into_iter().flat_map(|i| i.into_iter()) {
            let defined_on_field = index.keys.len() == 1;

            // Implicit primary key
            if matches!(index.keys.keys().next().map(Deref::deref), Some("_id")) {
                continue;
            }

            // Partial index
            if index
                .options
                .as_ref()
                .and_then(|opts| opts.partial_filter_expression.as_ref())
                .is_some()
            {
                continue;
            }

            // Points to a field that does not exist (yet).
            if !index.keys.keys().all(|k| {
                model
                    .fields
                    .iter()
                    .any(|f| f.name() == k || f.database_name() == Some(k))
            }) {
                continue;
            }

            let tpe = index
                .options
                .as_ref()
                .and_then(|opts| opts.unique)
                .map(|uniq| if uniq { IndexType::Unique } else { IndexType::Normal })
                .unwrap_or(IndexType::Normal);

            let db_name = index.options.and_then(|opts| opts.name);

            let fields = index
                .keys
                .into_iter()
                .map(|(k, _)| match sanitize_string(&k) {
                    Some(sanitized) => sanitized,
                    None => k,
                })
                .collect();

            model.add_index(IndexDefinition {
                fields,
                tpe,
                defined_on_field,
                name: None,
                db_name,
            });
        }
    }
}

fn sanitize_string(s: &str) -> Option<String> {
    static RE_START: Lazy<Regex> = Lazy::new(|| Regex::new("^[^a-zA-Z]+").unwrap());
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new("[^_a-zA-Z0-9]").unwrap());

    let needs_sanitation = RE_START.is_match(s) || RE.is_match(s);

    if needs_sanitation {
        let start_cleaned: String = RE_START.replace_all(s, "").parse().unwrap();
        let sanitized: String = RE.replace_all(start_cleaned.as_str(), "_").parse().unwrap();

        Some(sanitized)
    } else {
        None
    }
}
