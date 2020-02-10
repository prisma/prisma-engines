use chrono::Utc;
use prisma_models::{ModelIdentifier, ModelRef, PrismaValue, RecordIdentifier};
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

    pub fn as_record_identifier(&self, id: ModelIdentifier) -> Option<RecordIdentifier> {
        let pairs: Vec<_> = id
            .data_source_fields()
            .filter_map(|dsf| {
                self.get_field_value(dsf.name.as_str())
                    .map(|val| (dsf.clone(), val.clone()))
            })
            .collect();

        Some(pairs.into())
    }
}
