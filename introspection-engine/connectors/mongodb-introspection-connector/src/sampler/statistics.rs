mod name;

use mongodb_schema_describer::{IndexFieldProperty, IndexWalker};
pub(crate) use name::Name;

use super::{field_type::FieldType, CompositeTypeDepth};
use convert_case::{Case, Casing};
use datamodel::{
    CompositeType, CompositeTypeField, CompositeTypeFieldType, Datamodel, DefaultValue, Field, IndexDefinition,
    IndexField, IndexType, Model, PrimaryKeyDefinition, PrimaryKeyField, ScalarField, ScalarType, SortOrder,
    ValueGenerator, WithDatabaseName,
};
use introspection_connector::Warning;
use mongodb::bson::{Bson, Document};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
};

pub(super) const SAMPLE_SIZE: i32 = 1000;

static RESERVED_NAMES: &[&str] = &["PrismaClient"];
static COMMENTED_OUT_FIELD: &str = "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*";
static EMPTY_TYPE_DETECTED: &str = "Nested objects had no data in the sample dataset to introspect a nested type.";

/// Statistical data from a MongoDB database for determining a Prisma data
/// model.
#[derive(Default)]
pub(super) struct Statistics<'a> {
    /// (container_name, field_name) -> type percentages
    samples: BTreeMap<(Name, String), FieldSampler>,
    /// model_name -> document count
    models: HashMap<Name, usize>,
    /// model_name -> indices
    indices: BTreeMap<String, Vec<IndexWalker<'a>>>,
    /// How deep we travel in nested composite types until switching to Json. None will always use
    /// Json, Some(-1) will never switch to Json.
    composite_type_depth: CompositeTypeDepth,
}

impl<'a> Statistics<'a> {
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
    pub(super) fn track_index(&mut self, model_name: &str, index: IndexWalker<'a>) {
        let indexes = self.indices.entry(model_name.to_string()).or_default();
        indexes.push(index);
    }

    /// From the given data, create a Prisma data model with best effort basis.
    pub(super) fn into_datamodel(self, warnings: &mut Vec<Warning>) -> Datamodel {
        let mut data_model = Datamodel::new();
        let mut indices = self.indices;
        let (mut models, types) = populate_fields(&self.models, self.samples, warnings);

        add_indices_to_models(&mut models, &mut indices, warnings);
        add_missing_ids_to_models(&mut models);

        for (_, model) in models.into_iter() {
            data_model.add_model(model);
        }

        for (_, mut composite_type) in types.into_iter() {
            if composite_type
                .fields
                .iter()
                .any(|f| f.database_name == Some("_id".into()))
            {
                if let Some(field) = composite_type
                    .fields
                    .iter_mut()
                    .find(|f| f.name == *"id" && f.database_name.is_none())
                {
                    field.name = "id_".into();
                    field.database_name = Some("id".into());
                }
            }

            data_model.composite_types.push(composite_type);
        }

        data_model
    }

    /// Creates a new name for a composite type with the following rules:
    ///
    /// - if model is foo and field is bar, the type is FooBar
    /// - if a model already exists with the name, we'll use FooBar_
    fn composite_type_name(&self, model: &str, field: &str) -> Name {
        let name = Name::Model(format!("{}_{}", model, field).to_case(Case::Pascal));

        let name = if self.models.contains_key(&name) {
            format!("{}_", name)
        } else {
            name.take()
        };

        Name::CompositeType(name)
    }

    /// Tracking the usage of types and names in a composite type.
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

    /// If a document has a nested document, we'll introspect it as a composite
    /// type until a certain depth. The depth can be given by user, and if we
    /// reach enough nesting the following composite types are introspected as
    /// `Json`.
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

            let sampler = self.samples.entry((name.clone(), field.to_string())).or_default();
            sampler.counter += 1;

            match FieldType::from_bson(val, compound_name) {
                // We cannot have arrays of arrays, so multi-dimensional arrays
                // are introspected as `Json`.
                Some(_) if array_layers > 1 => {
                    let counter = sampler.types.entry(FieldType::Json).or_default();
                    *counter += 1;
                }
                // Counting the types.
                Some(field_type) => {
                    let counter = sampler.types.entry(field_type).or_default();
                    *counter += 1;
                }
                // If the value is null, the field must be optional and we
                // cannot detect the type.
                None => {
                    sampler.nullable = true;
                }
            }
        }
    }
}

/// A document must have a id column and the name is always `_id`. If we have no
/// data in the collection, we must assume an id field exists.
fn add_missing_ids_to_models(models: &mut BTreeMap<String, Model>) {
    for (_, model) in models.iter_mut() {
        if model.fields.iter().any(|f| f.database_name() == Some("_id")) {
            continue;
        }

        let field = ScalarField {
            name: String::from("id"),
            field_type: datamodel::FieldType::from(FieldType::ObjectId),
            arity: datamodel::FieldArity::Required,
            database_name: Some(String::from("_id")),
            default_value: Some(DefaultValue::new_expression(ValueGenerator::new_auto())),
            documentation: None,
            is_generated: false,
            is_updated_at: false,
            is_commented_out: false,
            is_ignored: false,
            comment_value: None,
        };

        model.fields.insert(0, Field::ScalarField(field));
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct FieldSampler {
    types: BTreeMap<FieldType, usize>,
    nullable: bool,
    counter: usize,
}

impl FieldSampler {
    /// Counting the percentages of different types per field.
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
        fields: vec![PrimaryKeyField {
            name: "id".to_string(),
            sort_order: None,
            length: None,
        }],
        defined_on_field: true,
    };

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
        fields: vec![],
        database_name,
        documentation,
        ..Default::default()
    }
}

/// Read all samples from the data, returning models and composite types.
///
/// ## Input
///
/// - Samples, counting how many documents altogether there was in the model or
///   in how many documents we had data for the composite type.
/// - Fields counts from model or type and field name combination to statistics
///   of different types seen in the data.
fn populate_fields(
    samples: &HashMap<Name, usize>,
    fields: BTreeMap<(Name, String), FieldSampler>,
    warnings: &mut Vec<Warning>,
) -> (BTreeMap<String, Model>, BTreeMap<String, CompositeType>) {
    let mut models: BTreeMap<String, Model> = samples
        .iter()
        .flat_map(|(name, _)| name.as_model_name())
        .map(|model_name| (model_name.to_string(), new_model(model_name)))
        .collect();

    let mut types: BTreeMap<String, CompositeType> = samples
        .iter()
        .flat_map(|(name, _)| name.as_type_name())
        .map(|type_name| (type_name.to_string(), new_composite_type(type_name)))
        .collect();

    let mut unsupported = Vec::new();
    let mut unknown_types = Vec::new();
    let mut undecided_types = Vec::new();
    let mut fields_with_empty_names = Vec::new();

    for ((container, field_name), sampler) in fields.into_iter() {
        let doc_count = *samples.get(&container).unwrap_or(&0);
        let field_count = sampler.counter;

        let percentages = sampler.percentages();
        let most_common_type = percentages.find_most_common();

        let field_type = match &most_common_type {
            Some(field_type) => field_type.to_owned(),
            None => FieldType::Json,
        };

        if let FieldType::Unsupported(r#type) = field_type {
            unsupported.push((container.clone(), field_name.to_string(), r#type));
        }

        if percentages.data.len() > 1 {
            undecided_types.push((container.clone(), field_name.to_string(), field_type.to_string()));
        }

        let arity = if field_type.is_array() {
            datamodel::FieldArity::List
        } else if doc_count > field_count || sampler.nullable {
            datamodel::FieldArity::Optional
        } else {
            datamodel::FieldArity::Required
        };

        let mut documentation = if percentages.has_type_variety() {
            Some(format!(
                "Multiple data types found: {} out of {} sampled entries",
                percentages, field_count
            ))
        } else {
            None
        };

        if most_common_type.is_none() {
            static UNKNOWN_FIELD: &str =
                "Could not determine type: the field only had null or empty values in the sample set.";

            match &mut documentation {
                Some(docs) => {
                    docs.push('\n');
                    docs.push_str(UNKNOWN_FIELD);
                }
                None => {
                    documentation = Some(UNKNOWN_FIELD.to_owned());
                }
            }

            unknown_types.push((container.clone(), field_name.to_string()));
        }

        let (name, database_name, is_commented_out) = match sanitize_string(&field_name) {
            Some(sanitized) if sanitized.is_empty() => {
                match documentation.as_mut() {
                    Some(ref mut existing) => {
                        existing.push('\n');
                        existing.push_str(COMMENTED_OUT_FIELD);
                    }
                    None => {
                        documentation = Some(COMMENTED_OUT_FIELD.to_string());
                    }
                };

                fields_with_empty_names.push((container.clone(), field_name.clone()));

                (field_name.clone(), Some(field_name), true)
            }
            Some(sanitized) => (sanitized, Some(field_name), false),
            None if matches!(container, Name::Model(_)) && field_name == "id" => {
                ("id_".to_string(), Some(field_name), false)
            }
            None => (field_name, None, false),
        };

        match container {
            Name::Model(model_name) => {
                let model = models.get_mut(&model_name).unwrap();

                let mut field = ScalarField {
                    name,
                    field_type: datamodel::FieldType::from(field_type.clone()),
                    arity,
                    database_name,
                    default_value: None,
                    documentation,
                    is_generated: false,
                    is_updated_at: false,
                    is_commented_out,
                    is_ignored: false,
                    comment_value: None,
                };

                match &field.database_name {
                    Some(name) if name == "_id" => {
                        if let FieldType::ObjectId = &field_type {
                            field.set_default_value(DefaultValue::new_expression(ValueGenerator::new_auto()));
                        };

                        model.fields.insert(0, Field::ScalarField(field));
                    }
                    _ => model.fields.push(Field::ScalarField(field)),
                };
            }
            Name::CompositeType(type_name) => {
                let r#type = types.get_mut(&type_name).unwrap();

                r#type.fields.push(CompositeTypeField {
                    name,
                    r#type: field_type.into(),
                    default_value: None,
                    arity,
                    documentation,
                    database_name,
                    is_commented_out,
                });
            }
        }
    }

    if !unsupported.is_empty() {
        warnings.push(crate::warnings::unsupported_type(&unsupported));
    }

    if !undecided_types.is_empty() {
        warnings.push(crate::warnings::undecided_field_type(&undecided_types));
    }

    if !fields_with_empty_names.is_empty() {
        warnings.push(crate::warnings::fields_with_empty_names(&fields_with_empty_names));
    }

    if !unknown_types.is_empty() {
        warnings.push(crate::warnings::fields_with_unknown_types(&unknown_types));
    }

    filter_out_empty_types(&mut models, &mut types, warnings);

    (models, types)
}

/// From the resulting data model, remove all types with no fields and change
/// the field types to Json.
fn filter_out_empty_types(
    models: &mut BTreeMap<String, Model>,
    types: &mut BTreeMap<String, CompositeType>,
    warnings: &mut Vec<Warning>,
) {
    let mut fields_with_an_empty_type = Vec::new();

    // 1. remove all types that have no fields.
    let empty_types: HashSet<_> = types
        .iter()
        .filter(|(_, r#type)| r#type.fields.is_empty())
        .map(|(name, _)| name.to_owned())
        .collect();

    // https://github.com/rust-lang/rust/issues/70530
    types.retain(|_, r#type| !r#type.fields.is_empty());

    // 2. change all fields in models that point to a non-existing type to Json.
    for (model_name, model) in models.iter_mut() {
        for field in model.fields.iter_mut().filter_map(|f| f.as_scalar_field_mut()) {
            match &field.field_type {
                datamodel::FieldType::CompositeType(ct) if empty_types.contains(ct) => {
                    fields_with_an_empty_type.push((Name::Model(model_name.clone()), field.name.clone()));
                    field.field_type = datamodel::FieldType::Scalar(datamodel::ScalarType::Json, None, None);
                    field.documentation = Some(EMPTY_TYPE_DETECTED.to_owned());
                }
                _ => (),
            }
        }
    }

    // 3. change all fields in types that point to a non-existing type to Json.
    for (type_name, r#type) in types.iter_mut() {
        for field in r#type.fields.iter_mut() {
            match &field.r#type {
                CompositeTypeFieldType::CompositeType(name) if empty_types.contains(name) => {
                    fields_with_an_empty_type.push((Name::CompositeType(type_name.clone()), field.name.clone()));
                    field.r#type = CompositeTypeFieldType::Scalar(ScalarType::Json, None, None);
                    field.documentation = Some(EMPTY_TYPE_DETECTED.to_owned());
                }
                _ => (),
            }
        }
    }

    // 4. add warnings in the end to reduce spam
    if !fields_with_an_empty_type.is_empty() {
        warnings.push(crate::warnings::fields_pointing_to_an_empty_type(
            &fields_with_an_empty_type,
        ));
    }
}

fn add_indices_to_models(
    models: &mut BTreeMap<String, Model>,
    indices: &mut BTreeMap<String, Vec<IndexWalker<'_>>>,
    warnings: &mut Vec<Warning>,
) {
    let mut fields_with_unknown_type = Vec::new();

    for (model_name, model) in models.iter_mut() {
        for index in indices.remove(model_name).into_iter().flat_map(|i| i.into_iter()) {
            let defined_on_field = index.fields().len() == 1;

            let missing_fields: Vec<_> = index
                .fields()
                .filter(|indf| {
                    !model
                        .fields
                        .iter()
                        .any(|mf| mf.name() == indf.name() || mf.database_name() == Some(indf.name()))
                })
                .cloned()
                .collect();

            for field in missing_fields {
                let docs = String::from("Field referred in an index, but found no data to define the type.");
                fields_with_unknown_type.push((Name::Model(model_name.to_owned()), field.name.clone()));

                let (name, database_name) = match sanitize_string(field.name()) {
                    Some(name) => (name, Some(field.name)),
                    None => (field.name, None),
                };

                let sf = ScalarField {
                    name,
                    field_type: FieldType::Json.into(),
                    arity: datamodel::FieldArity::Optional,
                    database_name,
                    default_value: None,
                    documentation: Some(docs),
                    is_generated: false,
                    is_updated_at: false,
                    is_commented_out: false,
                    is_ignored: false,
                    comment_value: None,
                };

                model.fields.push(Field::ScalarField(sf));
            }

            let fields = index
                .fields()
                .map(|f| IndexField {
                    name: sanitize_string(f.name()).unwrap_or_else(|| f.name().to_string()),
                    sort_order: match f.property {
                        IndexFieldProperty::Text => None,
                        IndexFieldProperty::Ascending => Some(SortOrder::Asc),
                        IndexFieldProperty::Descending => Some(SortOrder::Desc),
                    },
                    length: None,
                })
                .collect();

            let tpe = match index.r#type() {
                mongodb_schema_describer::IndexType::Normal => IndexType::Normal,
                mongodb_schema_describer::IndexType::Unique => IndexType::Unique,
                mongodb_schema_describer::IndexType::Fulltext => IndexType::Fulltext,
            };

            model.add_index(IndexDefinition {
                fields,
                tpe,
                defined_on_field,
                db_name: Some(index.name().to_string()),
                name: None,
                algorithm: None,
            });
        }
    }

    if !fields_with_unknown_type.is_empty() {
        warnings.push(crate::warnings::fields_with_unknown_types(&fields_with_unknown_type));
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
