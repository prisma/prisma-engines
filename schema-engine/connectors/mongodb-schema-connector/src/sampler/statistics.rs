mod indices;
mod name;

pub(crate) use name::Name;
use renderer::{
    datamodel::{IdFieldDefinition, UniqueFieldAttribute},
    value::Function,
};
use schema_connector::{
    warnings::{ModelAndField, ModelAndFieldAndType, TypeAndField, TypeAndFieldAndType},
    CompositeTypeDepth, IntrospectionContext, Warnings,
};

use super::field_type::FieldType;
use convert_case::{Case, Casing};
use datamodel_renderer as renderer;
use mongodb::bson::{Bson, Document};
use mongodb_schema_describer::{CollectionWalker, IndexWalker};
use once_cell::sync::Lazy;
use psl::datamodel_connector::constraint_names::ConstraintNames;
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

#[derive(Default, Clone, Copy)]
struct ModelData<'a> {
    document_count: usize,
    has_id: bool,
    collection_walker: Option<CollectionWalker<'a>>,
}

/// Statistical data from a MongoDB database for determining a Prisma data
/// model.
#[derive(Default)]
pub(super) struct Statistics<'a> {
    /// (container_name, field_name) -> type percentages
    samples: BTreeMap<(Name, String), FieldSampler>,
    /// Container for composite types that are not empty
    types_with_fields: HashSet<String>,
    /// model_name -> document count
    models: HashMap<Name, ModelData<'a>>,
    /// model_name -> indices
    indices: BTreeMap<String, Vec<IndexWalker<'a>>>,
    /// How deep we travel in nested composite types until switching to Json. None will always use
    /// Json, Some(-1) will never switch to Json.
    composite_type_depth: CompositeTypeDepth,
}

impl<'a> Statistics<'a> {
    /// Creates a new name for a composite type with the following rules:
    ///
    /// - if model is foo and field is bar, the type is FooBar
    /// - if a model already exists with the name, we'll use FooBar_
    fn composite_type_name(&self, model: &str, field: &str) -> Name {
        let combined: String = format!("{model}_{field}").chars().filter(|c| c.is_ascii()).collect();

        let name = Name::Model(combined.to_case(Case::Pascal));

        let name = if self.models.contains_key(&name) {
            format!("{name}_")
        } else {
            name.take()
        };

        Name::CompositeType(name)
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

    pub(super) fn new(composite_type_depth: CompositeTypeDepth) -> Self {
        Self {
            composite_type_depth,
            ..Default::default()
        }
    }

    pub(super) fn render(
        &'a self,
        ctx: &'a IntrospectionContext,
        rendered: &mut renderer::Datamodel<'a>,
        warnings: &mut Warnings,
    ) {
        let mut models: BTreeMap<&str, renderer::datamodel::Model<'_>> = self
            .models
            .iter()
            .flat_map(|(name, doc_count)| name.as_model_name().map(|name| (name, doc_count)))
            .map(|(name, doc_count)| {
                let mut model = match sanitize_string(name) {
                    Some(sanitized) => {
                        let mut model = renderer::datamodel::Model::new(sanitized);
                        model.map(name);
                        model
                    }
                    None if RESERVED_NAMES.contains(&name) => {
                        let mut model = renderer::datamodel::Model::new(format!("Renamed{name}"));
                        model.map(name);

                        let docs = format!("This model has been renamed to 'Renamed{name}' during introspection, because the original name '{name}' is reserved.");
                        model.documentation(docs);

                        model
                    }
                    None => renderer::datamodel::Model::new(name),
                };

                if let Some(walker) = doc_count.collection_walker {
                    if walker.has_schema() {
                        let comment = "This collection uses a JSON Schema defined in the database, which requires additional setup for migrations. Visit https://pris.ly/d/mongodb-json-schema for more info.";
                        model.documentation(comment)
                    }

                    if walker.is_capped() {
                        let comment = "This model is a capped collection, which is not yet fully supported. Read more: https://pris.ly/d/mongodb-capped-collections";
                        model.documentation(comment)
                    }
                }

                if !doc_count.has_id {
                    let mut field = renderer::datamodel::Field::new("id", "String");

                    field.map("_id");
                    field.native_type(&ctx.datasource().name, "ObjectId", Vec::new());
                    field.default(renderer::datamodel::DefaultValue::function(Function::new("auto")));
                    field.id(IdFieldDefinition::new());

                    model.push_field(field);
                }

                (name, model)
            })
            .collect();

        let mut types: BTreeMap<&str, renderer::datamodel::CompositeType<'_>> = self
            .models
            .iter()
            .flat_map(|(name, _)| name.as_type_name())
            .filter(|name| self.types_with_fields.contains(*name))
            .map(|name| (name, renderer::datamodel::CompositeType::new(name)))
            .collect();

        for (model_name, indices) in self.indices.iter() {
            let model = models.get_mut(model_name.as_str()).unwrap();

            let indices = indices.iter().filter(|idx| {
                !idx.is_unique() || idx.fields().len() > 1 || idx.fields().any(|f| f.name().contains('.'))
            });

            indices::render(model, model_name, indices);
        }

        for ((container, field_name), sampler) in self.samples.iter() {
            let doc_count = *self.models.get(container).unwrap_or(&Default::default());

            let field_count = sampler.counter;

            let percentages = sampler.percentages();
            let most_common_type = percentages.find_most_common();
            let no_known_type = most_common_type.is_none();

            let sanitized = sanitize_string(field_name);

            let points_to_an_empty_type = most_common_type
                .as_ref()
                .map(|t| t.is_document() && !types.contains_key(t.prisma_type()))
                .unwrap_or(false);

            let field_type = match most_common_type {
                Some(field_type) if !percentages.has_type_variety() => {
                    let prisma_type = field_type.prisma_type();

                    if !field_type.is_document() || types.contains_key(prisma_type) {
                        field_type
                    } else {
                        FieldType::Json
                    }
                }
                Some(_) if percentages.all_types_are_datetimes() => FieldType::Timestamp,
                _ => FieldType::Json,
            };

            let prisma_type = field_type.prisma_type();

            let mut field = match sanitized {
                Some(sanitized) if sanitized.is_empty() => {
                    match container {
                        Name::Model(name) => warnings.fields_with_empty_names_in_model.push(ModelAndField {
                            model: name.to_string(),
                            field: field_name.to_string(),
                        }),
                        Name::CompositeType(name) => warnings.fields_with_empty_names_in_type.push(TypeAndField {
                            composite_type: name.to_string(),
                            field: field_name.to_string(),
                        }),
                    }

                    let mut field = renderer::datamodel::Field::new(field_name, prisma_type.to_string());
                    field.map(field_name);
                    field.documentation(COMMENTED_OUT_FIELD);
                    field.commented_out();

                    field
                }
                Some(sanitized) => {
                    let mut field = renderer::datamodel::Field::new(sanitized, prisma_type.to_string());
                    field.map(field_name);

                    field
                }
                None if doc_count.has_id && field_name == "id" => {
                    let mut field = renderer::datamodel::Field::new("id_", prisma_type.to_string());
                    field.map(field_name);

                    field
                }
                None => renderer::datamodel::Field::new(field_name, prisma_type.to_string()),
            };

            if points_to_an_empty_type {
                let docs = "Nested objects had no data in the sample dataset to introspect a nested type.";
                field.documentation(docs);

                match container {
                    Name::Model(name) => warnings.model_fields_pointing_to_an_empty_type.push(ModelAndField {
                        model: name.to_string(),
                        field: field_name.to_string(),
                    }),
                    Name::CompositeType(name) => warnings.type_fields_pointing_to_an_empty_type.push(TypeAndField {
                        composite_type: name.to_string(),
                        field: field_name.to_string(),
                    }),
                }
            }

            if field_name == "_id" && !container.is_composite_type() {
                field.id(IdFieldDefinition::default());

                if matches!(field_type, FieldType::ObjectId) {
                    field.default(renderer::datamodel::DefaultValue::function(Function::new("auto")));
                }
            }

            if let Some(native_type) = field_type.native_type() {
                field.native_type(&ctx.datasource().name, native_type.to_string(), Vec::new());
            }

            if field_type.is_array() {
                field.array();
            } else if doc_count.document_count > field_count || sampler.nullable {
                field.optional();
            }

            if field_type.is_unsupported() {
                field.unsupported();

                match container {
                    Name::Model(name) => warnings.unsupported_types_in_model.push(ModelAndFieldAndType {
                        model: name.to_string(),
                        field: field_name.to_string(),
                        r#type: field_type.prisma_type().to_string(),
                    }),
                    Name::CompositeType(name) => warnings.unsupported_types_in_type.push(TypeAndFieldAndType {
                        composite_type: name.to_string(),
                        field: field_name.to_string(),
                        r#type: field_type.prisma_type().to_string(),
                    }),
                }
            }

            if percentages.data.len() > 1 {
                match container {
                    Name::Model(name) => warnings.undecided_types_in_models.push(ModelAndFieldAndType {
                        model: name.to_string(),
                        field: field_name.to_string(),
                        r#type: field_type.to_string(),
                    }),
                    Name::CompositeType(name) => warnings.undecided_types_in_types.push(TypeAndFieldAndType {
                        composite_type: name.to_string(),
                        field: field_name.to_string(),
                        r#type: field_type.to_string(),
                    }),
                }
            }

            if sampler.from_index {
                let docs = "Field referred in an index, but found no data to define the type.";
                field.documentation(docs);

                match container {
                    Name::Model(name) => warnings.model_fields_with_unknown_type.push(ModelAndField {
                        model: name.to_string(),
                        field: field_name.to_string(),
                    }),
                    Name::CompositeType(name) => warnings.type_fields_with_unknown_type.push(TypeAndField {
                        composite_type: name.to_string(),
                        field: field_name.to_string(),
                    }),
                }
            }

            if percentages.has_type_variety() {
                let doc = format!("Multiple data types found: {percentages} out of {field_count} sampled entries",);
                field.documentation(doc);
            }

            if no_known_type {
                let doc = "Could not determine type: the field only had null or empty values in the sample set.";
                field.documentation(doc);

                match container {
                    Name::Model(name) => warnings.model_fields_with_unknown_type.push(ModelAndField {
                        model: name.to_string(),
                        field: field_name.to_string(),
                    }),
                    Name::CompositeType(name) => warnings.type_fields_with_unknown_type.push(TypeAndField {
                        composite_type: name.to_string(),
                        field: field_name.to_string(),
                    }),
                }
            }

            match container {
                Name::Model(ref model_name) => {
                    let unique = self.indices.get(model_name).and_then(|indices| {
                        indices.iter().find(|idx| {
                            idx.is_unique() && idx.fields().len() == 1 && idx.fields().any(|f| f.name() == field_name)
                        })
                    });

                    if let Some(unique) = unique {
                        let index_field = unique.fields().next().unwrap();
                        let mut attr = UniqueFieldAttribute::default();

                        let default_name = ConstraintNames::unique_index_name(
                            model_name,
                            &[index_field.name()],
                            psl::builtin_connectors::MONGODB,
                        );

                        if unique.name() != default_name {
                            attr.map(unique.name());
                        };

                        if index_field.property.is_descending() {
                            attr.sort_order("Desc")
                        }

                        field.unique(attr);
                    }

                    let model = models.get_mut(model_name.as_str()).unwrap();

                    if field_name == "_id" {
                        model.insert_field_front(field);
                    } else {
                        model.push_field(field);
                    }
                }
                Name::CompositeType(ref type_name) => {
                    let r#type = types
                        .entry(type_name.as_str())
                        .or_insert_with(|| renderer::datamodel::CompositeType::new(type_name));

                    r#type.push_field(field);
                }
            }
        }

        for (ct_name, r#type) in types {
            let file_name = match ctx.previous_schema().db.find_composite_type(ct_name) {
                Some(walker) => Cow::Borrowed(ctx.previous_schema().db.file_name(walker.file_id())),
                None => ctx.introspection_file_path(),
            };

            rendered.push_composite_type(file_name, r#type);
        }

        for (model_name, model) in models.into_iter() {
            let file_name = match ctx.previous_schema().db.find_model(model_name) {
                Some(walker) => Cow::Borrowed(ctx.previous_schema().db.file_name(walker.file_id())),
                None => ctx.introspection_file_path(),
            };

            rendered.push_model(file_name, model);
        }
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

    /// Track all fields and field types from the given document.
    fn track_document_types(&mut self, name: Name, document: &Document, depth: CompositeTypeDepth) {
        if name.is_composite_type() && depth.is_none() {
            return;
        }

        let doc_count = self.models.entry(name.clone()).or_default();
        doc_count.document_count += 1;
        doc_count.has_id = document.iter().any(|(key, _)| key == "_id");

        let depth = match name {
            Name::CompositeType(_) => depth.level_down(),
            _ => depth,
        };

        match name {
            Name::CompositeType(ref name) if !document.is_empty() => {
                self.types_with_fields.insert(name.to_string());
            }
            _ => (),
        }

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

    /// Track an index for the given model.
    pub(super) fn track_index(&mut self, model_name: &str, index: IndexWalker<'a>) {
        for field in index.fields() {
            if field.name().contains('.') {
                let path_len = field.name().split('.').count();
                let path = field.name().split('.');
                let mut container_name = model_name.to_string();

                for (i, field) in path.enumerate() {
                    let field = field.to_string();

                    let key = if i == 0 {
                        (Name::Model(container_name.clone()), field.clone())
                    } else {
                        self.types_with_fields.insert(container_name.clone());
                        let name = Name::CompositeType(container_name.clone());

                        if self.models.contains_key(&name) {
                            self.models.insert(name.clone(), Default::default());
                        }

                        (name, field.clone())
                    };

                    let type_name = format!("{container_name}_{field}").to_case(Case::Pascal);
                    let type_name = sanitize_string(&type_name).unwrap_or(type_name);
                    container_name.clone_from(&type_name);

                    if let Some(sampler) = self.samples.get_mut(&key) {
                        let has_composites = sampler.types.iter().any(|t| t.0.has_documents());

                        if i < path_len - 1 && !has_composites {
                            let counter = sampler.types.entry(FieldType::Document(type_name.clone())).or_default();
                            *counter += 1;
                        }

                        continue;
                    }

                    let mut sampler = if i < path_len - 1 {
                        let mut sampler = FieldSampler::default();
                        sampler.types.insert(FieldType::Document(type_name.clone()), 1);

                        let key = Name::CompositeType(type_name);
                        self.models.entry(key).or_default();

                        sampler
                    } else {
                        let mut sampler = FieldSampler::default();
                        sampler.types.insert(FieldType::Json, 1);

                        sampler
                    };

                    sampler.from_index = true;
                    sampler.nullable = true;
                    sampler.counter = 1;

                    self.samples.insert(key, sampler);
                }
            } else {
                let key = (Name::Model(model_name.to_string()), field.name().to_string());

                if self.samples.contains_key(&key) {
                    continue;
                }

                let mut sampler = FieldSampler::default();
                sampler.types.insert(FieldType::Json, 1);
                sampler.from_index = true;
                sampler.nullable = true;
                sampler.counter = 1;

                self.samples.insert(key, sampler);
            }
        }

        let indexes = self.indices.entry(model_name.to_string()).or_default();
        indexes.push(index);
    }

    /// Track a collection as prisma model.
    pub(super) fn track_model(&mut self, model: &str, collection: CollectionWalker<'a>) {
        let model = self.models.entry(Name::Model(model.to_string())).or_default();
        model.collection_walker = Some(collection);
    }

    pub(super) fn track_model_fields(&mut self, model: &str, document: Document) {
        self.track_document_types(Name::Model(model.to_string()), &document, self.composite_type_depth);
    }
}

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct FieldSampler {
    types: BTreeMap<FieldType, usize>,
    from_index: bool,
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
            write!(f, "{k}: {p}%")?;

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

    /// All instances we found were either of `Date` or `Timestamp` type.
    fn all_types_are_datetimes(&self) -> bool {
        self.data
            .iter()
            .all(|(typ, _)| matches!(typ, FieldType::Date | FieldType::Timestamp))
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
