use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fmt,
};

use bson::Document;
use datamodel::{
    Datamodel, DefaultValue, Field, Model, NativeTypeInstance, PrimaryKeyDefinition, ScalarField, ScalarType,
    ValueGenerator,
};
use native_types::MongoDbType;

use super::field_type::FieldType;

#[derive(Debug, Default)]
pub(super) struct Statistics {
    fields: BTreeMap<(String, String), FieldSampler>,
    documents: HashMap<String, usize>,
}

impl Statistics {
    pub(super) fn track(&mut self, model: &str, document: Document) {
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

impl From<Statistics> for Datamodel {
    fn from(stats: Statistics) -> Self {
        let mut data_model = Datamodel::new();
        let mut models: BTreeMap<String, Model> = BTreeMap::new();

        for ((model_name, field_name), sampler) in stats.fields.into_iter() {
            let doc_count = *stats.documents.get(&model_name).unwrap_or(&0);
            let field_count = sampler.counter;

            let model = models.entry(model_name.clone()).or_insert_with(|| {
                let primary_key = PrimaryKeyDefinition {
                    name: None,
                    db_name: None,
                    fields: vec!["id".to_string()],
                    defined_on_field: true,
                };

                let primary_key_field = Field::ScalarField(ScalarField {
                    name: "id".to_string(),
                    field_type: datamodel::FieldType::Scalar(
                        ScalarType::String,
                        None,
                        Some(NativeTypeInstance::new("ObjectId", Vec::new(), &MongoDbType::ObjectId)),
                    ),
                    arity: datamodel::FieldArity::Required,
                    database_name: Some("_id".to_string()),
                    default_value: Some(DefaultValue::new_expression(
                        ValueGenerator::new("dbgenerated".to_owned(), Vec::new()).unwrap(),
                    )),
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                    is_commented_out: false,
                    is_ignored: false,
                });

                Model {
                    name: model_name,
                    primary_key: Some(primary_key),
                    fields: vec![primary_key_field],
                    ..Default::default()
                }
            });

            if field_name == "_id" {
                continue;
            }

            let percentages = sampler.percentages();
            let field_type = percentages.find_most_common().unwrap().to_owned();

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

        data_model
    }
}
