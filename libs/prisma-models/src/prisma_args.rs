use crate::prelude::{ModelRef, PrismaValue};
use chrono::Utc;
use std::collections::{btree_map::Keys, BTreeMap};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct PrismaArgs {
    pub args: BTreeMap<String, PrismaValue>,
}

impl From<BTreeMap<String, PrismaValue>> for PrismaArgs {
    fn from(args: BTreeMap<String, PrismaValue>) -> Self {
        Self { args }
    }
}

impl PrismaArgs {
    pub fn new() -> Self {
        Self { args: BTreeMap::new() }
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
                self.args.insert(f.name.clone(), now.clone());
            }
        }

        if let Some(f) = updated_at_field {
            if let None = self.args.get(&f.name) {
                self.args.insert(f.name.clone(), now.clone());
            }
        }
    }

    pub fn update_datetimes(&mut self, model: ModelRef, list_causes_update: bool) {
        if !self.args.is_empty() || list_causes_update {
            if let Some(field) = model.fields().updated_at() {
                if let None = self.args.get(&field.name) {
                    self.args.insert(field.name.clone(), PrismaValue::DateTime(Utc::now()));
                }
            }
        }
    }
}
