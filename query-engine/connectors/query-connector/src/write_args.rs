use chrono::Utc;
use prisma_models::{ModelRef, PrismaValue};
use std::collections::{btree_map::Keys, BTreeMap};

/// A FieldValueContainer encapulates one or more values depending on
/// the field it belongs to, as fields may have more than one underlying
/// database field, most notably relation fields (multi-col fks for example).
#[derive(Debug, PartialEq, Clone)]
pub enum FieldValueContainer {
    Single(PrismaValue),
    Compound(Vec<PrismaValue>),
}

/// WriteArgs represent data to be written to an underlying data source.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct WriteArgs {
    pub args: BTreeMap<String, FieldValueContainer>,
}

impl From<BTreeMap<String, FieldValueContainer>> for WriteArgs {
    fn from(args: BTreeMap<String, FieldValueContainer>) -> Self {
        Self { args }
    }
}

impl WriteArgs {
    pub fn new() -> Self {
        Self { args: BTreeMap::new() }
    }

    pub fn insert<T, V>(&mut self, key: T, arg: V)
    where
        T: Into<String>,
        V: Into<PrismaValue>,
    {
        self.args.insert(key.into(), FieldValueContainer::Single(arg.into()));
    }

    pub fn insert_compound<T, V>(&mut self, key: T, arg: Vec<V>)
    where
        T: Into<String>,
        V: Into<PrismaValue>,
    {
        let arg = arg.into_iter().map(Into::into).collect();
        self.args.insert(key.into(), FieldValueContainer::Compound(arg));
    }

    pub fn has_arg_for(&self, field: &str) -> bool {
        self.args.contains_key(field)
    }

    pub fn get_field_value(&self, field: &str) -> Option<&FieldValueContainer> {
        self.args.get(field)
    }

    pub fn take_field_value(&mut self, field: &str) -> Option<FieldValueContainer> {
        self.args.remove(field)
    }

    pub fn keys(&self) -> Keys<String, FieldValueContainer> {
        self.args.keys()
    }

    pub fn add_datetimes(&mut self, model: ModelRef) {
        let now = PrismaValue::DateTime(Utc::now());
        let created_at_field = model.fields().created_at();
        let updated_at_field = model.fields().updated_at();

        if let Some(f) = created_at_field {
            if let None = self.args.get(&f.name) {
                self.insert(f.name.clone(), now.clone());
            }
        }

        if let Some(f) = updated_at_field {
            if let None = self.args.get(&f.name) {
                self.insert(f.name.clone(), now.clone());
            }
        }
    }

    pub fn update_datetimes(&mut self, model: ModelRef) {
        if !self.args.is_empty() {
            if let Some(field) = model.fields().updated_at() {
                if let None = self.args.get(&field.name) {
                    self.insert(field.name.clone(), PrismaValue::DateTime(Utc::now()));
                }
            }
        }
    }
}
