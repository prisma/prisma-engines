use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fmt,
    ops::Deref,
};

use bson::Document;
use datamodel::{
    Datamodel, DefaultValue, Field, IndexDefinition, IndexType, Model, NativeTypeInstance, PrimaryKeyDefinition,
    ScalarField, ScalarType, ValueGenerator, WithDatabaseName,
};
use introspection_connector::Warning;
use mongodb::IndexModel;
use native_types::MongoDbType;

use super::field_type::FieldType;

#[derive(Debug, Default)]
pub(super) struct Statistics {
    fields: BTreeMap<(String, String), FieldSampler>,
    documents: HashMap<String, usize>,
    indices: BTreeMap<String, Vec<IndexModel>>,
}

impl Statistics {
    pub(super) fn track_document_types(&mut self, model: &str, document: Document) {
        let doc_count = self.documents.entry(model.to_string()).or_default();
        *doc_count += 1;

        for (field, val) in document.into_iter() {
            let sampler = self.fields.entry((model.to_string(), field.to_string())).or_default();
            sampler.counter += 1;

            match FieldType::from_bson(val) {
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

    pub(super) fn track_index(&mut self, model_name: &str, index: IndexModel) {
        let indexes = self.indices.entry(model_name.to_string()).or_default();
        indexes.push(index);
    }

    pub(super) fn into_datamodel(self, warnings: &mut Vec<Warning>) -> Datamodel {
        let mut data_model = Datamodel::new();
        let mut indices = self.indices;
        let mut unsupported = Vec::new();
        let mut undecided_types = Vec::new();

        let mut models: BTreeMap<String, Model> = self
            .documents
            .iter()
            .map(|(model_name, _)| (model_name.to_string(), new_model(model_name, &mut indices)))
            .collect();

        for ((model_name, field_name), sampler) in self.fields.into_iter() {
            let doc_count = *self.documents.get(&model_name).unwrap_or(&0);
            let field_count = sampler.counter;

            let model = models.get_mut(&model_name).unwrap();

            if field_name == "_id" {
                continue;
            }

            if field_name == "id" {
                warnings.push(crate::warnings::explicit_id_column(&model_name));
            }

            let percentages = sampler.percentages();
            let field_type = percentages.find_most_common().unwrap().to_owned();

            if let FieldType::Unsupported(r#type) = field_type {
                unsupported.push((model_name.to_string(), field_name.to_string(), r#type));
            }

            if percentages.data.len() > 1 {
                undecided_types.push((model_name.to_string(), field_name.to_string(), field_type.to_string()));
            }

            let arity = if field_type.is_array() {
                datamodel::FieldArity::List
            } else if doc_count > field_count || sampler.nullable {
                datamodel::FieldArity::Optional
            } else {
                datamodel::FieldArity::Required
            };

            let documentation = if percentages.has_type_variety() {
                Some(format!("{}", percentages))
            } else {
                None
            };

            model.fields.push(Field::ScalarField(ScalarField {
                name: field_name,
                field_type: field_type.into(),
                arity,
                database_name: None,
                default_value: None,
                documentation,
                is_generated: false,
                is_updated_at: false,
                is_commented_out: false,
                is_ignored: false,
            }))
        }

        for (_, model) in models.into_iter() {
            data_model.add_model(model);
        }

        if !unsupported.is_empty() {
            warnings.push(crate::warnings::unsupported_type(&unsupported));
        }

        if !undecided_types.is_empty() {
            warnings.push(crate::warnings::undecided_field_type(&undecided_types));
        }

        data_model
    }
}

#[derive(Debug, Default)]
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
    fn find_most_common(&self) -> Option<FieldType> {
        self.data
            .iter()
            .max_by(|left, right| left.1.partial_cmp(right.1).unwrap_or(Ordering::Equal))
            .map(|(r#type, _)| r#type.clone())
    }

    fn has_type_variety(&self) -> bool {
        self.data.len() > 1
    }
}

fn new_model(model_name: &str, indices: &mut BTreeMap<String, Vec<IndexModel>>) -> Model {
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

    let mut model = Model {
        name: model_name.to_string(),
        primary_key: Some(primary_key),
        fields: vec![primary_key_field],
        ..Default::default()
    };

    for index in indices.remove(model_name).into_iter().flat_map(|i| i.into_iter()) {
        let defined_on_field = index.keys.len() == 1;

        if matches!(index.keys.keys().next().map(Deref::deref), Some("_id")) {
            continue;
        }

        let tpe = index
            .options
            .as_ref()
            .and_then(|opts| opts.unique)
            .map(|uniq| if uniq { IndexType::Unique } else { IndexType::Normal })
            .unwrap_or(IndexType::Normal);

        let db_name = index.options.and_then(|opts| opts.name);

        let definition = IndexDefinition {
            fields: index.keys.into_iter().map(|(k, _)| k).collect(),
            tpe,
            defined_on_field,
            name: None,
            db_name,
        };

        model.add_index(definition);
    }

    model
}
