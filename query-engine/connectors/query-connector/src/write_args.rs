use crate::error::{ConnectorError, ErrorKind};
use chrono::Utc;
use prisma_models::{ModelProjection, ModelRef, PrismaValue, RecordProjection, ScalarFieldRef};
use std::{
    borrow::Borrow,
    collections::{hash_map::Keys, HashMap},
    convert::TryInto,
    ops::Deref,
};

/// WriteArgs represent data to be written to an underlying data source.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct WriteArgs {
    pub args: HashMap<DatasourceFieldName, WriteExpression>,
}

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
/// Wrapper struct to force a bit of a reflection whether or not the string passed
/// to the write arguments is the data source field name, not the model field name.
/// Also helps to avoid errors with convenient from-field conversions.
pub struct DatasourceFieldName(pub String);

impl Deref for DatasourceFieldName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<str> for DatasourceFieldName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&ScalarFieldRef> for DatasourceFieldName {
    fn from(sf: &ScalarFieldRef) -> Self {
        DatasourceFieldName(sf.db_name().to_owned())
    }
}

/// A WriteExpression allows to express more complex operations on how the data is written,
/// like field or inter-field arithmetic.
#[derive(Debug, PartialEq, Clone)]
pub enum WriteExpression {
    /// Reference to another field on the same model.
    Field(DatasourceFieldName),

    /// Write plain value to field.
    Value(PrismaValue),

    /// Add value to field.
    Add(PrismaValue),

    /// Substract value from field
    Substract(PrismaValue),

    /// Multiply field by value.
    Multiply(PrismaValue),

    /// Divide field by value.
    Divide(PrismaValue),
}

impl From<PrismaValue> for WriteExpression {
    fn from(pv: PrismaValue) -> Self {
        WriteExpression::Value(pv)
    }
}

impl TryInto<PrismaValue> for WriteExpression {
    type Error = ConnectorError;

    fn try_into(self) -> Result<PrismaValue, Self::Error> {
        match self {
            WriteExpression::Value(pv) => Ok(pv),
            x => Err(ConnectorError::from_kind(ErrorKind::InternalConversionError(format!(
                "Unable to convert write expression {:?} into prisma value.",
                x
            )))),
        }
    }
}

impl From<HashMap<DatasourceFieldName, PrismaValue>> for WriteArgs {
    fn from(args: HashMap<DatasourceFieldName, PrismaValue>) -> Self {
        Self {
            args: args.into_iter().map(|(k, v)| (k, WriteExpression::Value(v))).collect(),
        }
    }
}

impl From<HashMap<DatasourceFieldName, WriteExpression>> for WriteArgs {
    fn from(args: HashMap<DatasourceFieldName, WriteExpression>) -> Self {
        Self { args }
    }
}

impl From<Vec<(DatasourceFieldName, PrismaValue)>> for WriteArgs {
    fn from(pairs: Vec<(DatasourceFieldName, PrismaValue)>) -> Self {
        Self {
            args: pairs.into_iter().map(|(k, v)| (k, WriteExpression::Value(v))).collect(),
        }
    }
}

impl From<Vec<(DatasourceFieldName, WriteExpression)>> for WriteArgs {
    fn from(pairs: Vec<(DatasourceFieldName, WriteExpression)>) -> Self {
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
        T: Into<DatasourceFieldName>,
        V: Into<WriteExpression>,
    {
        self.args.insert(key.into(), arg.into());
    }

    pub fn has_arg_for(&self, field: &str) -> bool {
        self.args.contains_key(field)
    }

    pub fn get_field_value(&self, field: &str) -> Option<&WriteExpression> {
        self.args.get(field)
    }

    pub fn take_field_value(&mut self, field: &str) -> Option<WriteExpression> {
        self.args.remove(field)
    }

    pub fn keys(&self) -> Keys<DatasourceFieldName, WriteExpression> {
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
            if self.args.get(f.db_name()).is_none() {
                self.args.insert(f.into(), now.clone().into());
            }
        }

        if let Some(f) = updated_at_field {
            if self.args.get(f.db_name()).is_none() {
                self.args.insert(f.into(), now.into());
            }
        }
    }

    pub fn update_datetimes(&mut self, model: ModelRef) {
        if !self.args.is_empty() {
            if let Some(field) = model.fields().updated_at() {
                if self.args.get(field.db_name()).is_none() {
                    self.args.insert(field.into(), PrismaValue::DateTime(Utc::now()).into());
                }
            }
        }
    }

    pub fn as_record_projection(&self, model_projection: ModelProjection) -> Option<RecordProjection> {
        let pairs: Vec<_> = model_projection
            .scalar_fields()
            .map(|field| {
                let val: PrismaValue = match self.get_field_value(field.db_name()) {
                    Some(val) => {
                        // Important: This causes write expressions that are not plain values to produce
                        // null values. At the moment, this function is used to extract an ID for
                        // create record calls, which only operate on plain values _for now_. As soon
                        // as that changes we need to revisit the whole ID extraction on create / update topic.
                        let p: Option<PrismaValue> = val.clone().try_into().ok();
                        match p {
                            Some(p) => p,
                            None => PrismaValue::Null,
                        }
                    }
                    None => PrismaValue::Null,
                };

                (field, val)
            })
            .collect();

        Some(pairs.into())
    }
}
