use chrono::Utc;
use prisma_models::{ModelProjection, ModelRef, PrismaValue, RecordProjection};
use std::collections::{hash_map::Keys, HashMap};

/// WriteArgs represent data to be written to an underlying data source.
/// The key is the data source field name, NOT the model field name.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct WriteArgs {
    pub args: HashMap<String, PrismaValue>,
}

impl From<HashMap<String, PrismaValue>> for WriteArgs {
    fn from(args: HashMap<String, PrismaValue>) -> Self {
        Self { args }
    }
}

impl From<Vec<(String, PrismaValue)>> for WriteArgs {
    fn from(pairs: Vec<(String, PrismaValue)>) -> Self {
        Self {
            args: pairs.into_iter().collect(),
        }
    }
}

impl WriteArgs {
    pub fn new() -> Self {
        Self { args: HashMap::new() }
    }

    pub fn insert<T, V>(&mut self, key: T, arg: V)
    where
        T: Into<String>,
        V: Into<PrismaValue>,
    {
        self.args.insert(key.into(), arg.into());
    }

    pub fn has_arg_for(&self, field: &str) -> bool {
        self.args.contains_key(field)
    }

    pub fn get_field_value(&self, field: &str) -> Option<&PrismaValue> {
        self.args.get(field)
    }

    pub fn take_field_value(&mut self, field: &str) -> Option<PrismaValue> {
        self.args.remove(field)
    }

    pub fn keys(&self) -> Keys<String, PrismaValue> {
        self.args.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn add_datetimes(&mut self, model: ModelRef) {
        let now = PrismaValue::DateTime(Utc::now());
        let created_at_field = model.fields().created_at();
        let updated_at_field = model.fields().updated_at();

        if let Some(f) = created_at_field {
            if let None = self.args.get(&f.name) {
                self.insert(f.db_name().clone(), now.clone());
            }
        }

        if let Some(f) = updated_at_field {
            if let None = self.args.get(&f.name) {
                self.insert(f.db_name().clone(), now.clone());
            }
        }
    }

    pub fn update_datetimes(&mut self, model: ModelRef) {
        if !self.args.is_empty() {
            if let Some(field) = model.fields().updated_at() {
                if let None = self.args.get(field.db_name()) {
                    self.insert(field.db_name().clone(), PrismaValue::DateTime(Utc::now()));
                }
            }
        }
    }

    pub fn as_record_projection(&self, model_projection: ModelProjection) -> Option<RecordProjection> {
        let pairs: Vec<_> = model_projection
            .scalar_fields()
            .map(|field| {
                let val = match self.get_field_value(field.db_name()) {
                    Some(val) => val.clone(),
                    None => PrismaValue::null(field.type_identifier.clone()),
                };

                (field.clone(), val.clone())
            })
            .collect();

        Some(pairs.into())
    }
}
