use crate::error::{ConnectorError, ErrorKind};
use chrono::Utc;
use indexmap::{map::Keys, IndexMap};
use prisma_models::{
    CompositeFieldRef, Field, ModelProjection, ModelRef, PrismaValue, ScalarFieldRef, SelectedField, SelectionResult,
};
use std::{borrow::Borrow, convert::TryInto, ops::Deref};

/// WriteArgs represent data to be written to an underlying data source.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct WriteArgs {
    pub args: IndexMap<DatasourceFieldName, WriteOperation>,
}

/// Wrapper struct to force a bit of a reflection whether or not the string passed
/// to the write arguments is the data source field name, not the model field name.
/// Also helps to avoid errors with convenient from-field conversions.
#[derive(Debug, PartialEq, Clone, Hash, Eq)]
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

impl From<&CompositeFieldRef> for DatasourceFieldName {
    fn from(cf: &CompositeFieldRef) -> Self {
        DatasourceFieldName(cf.db_name().to_owned())
    }
}

/// A WriteExpression allows to express more complex operations on how the data is written,
/// like field or inter-field arithmetic.
#[derive(Debug, PartialEq, Clone)]
pub enum WriteOperation {
    Scalar(ScalarWriteOperation),
    Composite(CompositeWriteOperation),
}

impl WriteOperation {
    pub fn scalar_set(pv: PrismaValue) -> Self {
        Self::Scalar(ScalarWriteOperation::Set(pv))
    }

    pub fn scalar_add(pv: PrismaValue) -> Self {
        Self::Scalar(ScalarWriteOperation::Add(pv))
    }

    pub fn scalar_substract(pv: PrismaValue) -> Self {
        Self::Scalar(ScalarWriteOperation::Substract(pv))
    }

    pub fn scalar_multiply(pv: PrismaValue) -> Self {
        Self::Scalar(ScalarWriteOperation::Multiply(pv))
    }

    pub fn scalar_divide(pv: PrismaValue) -> Self {
        Self::Scalar(ScalarWriteOperation::Divide(pv))
    }

    pub fn composite_set(pv: PrismaValue) -> Self {
        Self::Composite(CompositeWriteOperation::Set(pv))
    }

    pub fn composite_update(writes: Vec<(DatasourceFieldName, WriteOperation)>) -> Self {
        Self::Composite(CompositeWriteOperation::Update(NestedWrite { writes }))
    }

    pub fn as_scalar(&self) -> Option<&ScalarWriteOperation> {
        if let Self::Scalar(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_composite(&self) -> Option<&CompositeWriteOperation> {
        if let Self::Composite(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn try_into_scalar(self) -> Result<ScalarWriteOperation, Self> {
        if let Self::Scalar(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }

    pub fn try_into_composite(self) -> Result<CompositeWriteOperation, Self> {
        if let Self::Composite(v) = self {
            Ok(v)
        } else {
            Err(self)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ScalarWriteOperation {
    /// Reference to another field on the same model (unused at the moment).
    Field(DatasourceFieldName),

    /// Write plain value to field.
    Set(PrismaValue),

    /// Add value to field.
    Add(PrismaValue),

    /// Substract value from field
    Substract(PrismaValue),

    /// Multiply field by value.
    Multiply(PrismaValue),

    /// Divide field by value.
    Divide(PrismaValue),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompositeWriteOperation {
    Set(PrismaValue),
    Unset,
    Update(NestedWrite),
}

#[derive(Debug, PartialEq, Clone)]
pub struct NestedWrite {
    pub writes: Vec<(DatasourceFieldName, WriteOperation)>,
}

impl NestedWrite {
    /// Unfolds nested writes into a flat list of `WriteOperation`s.
    pub fn unfold<'a>(self, field: &'a Field) -> Vec<(WriteOperation, &'a Field, String)> {
        self.unfold_impl(field, &mut vec![])
    }

    fn unfold_impl<'a>(self, field: &'a Field, path: &mut Vec<String>) -> Vec<(WriteOperation, &'a Field, String)> {
        let mut nested_writes: Vec<(WriteOperation, &'a Field, String)> = vec![];

        for (DatasourceFieldName(db_name), write) in self.writes {
            let nested_field = field
                .as_composite()
                .unwrap()
                .typ
                .find_field_by_db_name(&db_name)
                .unwrap();

            match write {
                WriteOperation::Composite(CompositeWriteOperation::Update(nested_write)) => {
                    let mut path = path.clone();
                    path.push(db_name);

                    nested_writes.extend(nested_write.unfold_impl(nested_field, &mut path));
                }
                _ => {
                    path.push(db_name);
                    nested_writes.push((write, nested_field, path.join(".").to_owned()));
                }
            }
        }

        nested_writes
    }
}

impl From<(&ScalarFieldRef, PrismaValue)> for WriteOperation {
    fn from((_, pv): (&ScalarFieldRef, PrismaValue)) -> Self {
        WriteOperation::scalar_set(pv)
    }
}

impl From<(&CompositeFieldRef, PrismaValue)> for WriteOperation {
    fn from((_, pv): (&CompositeFieldRef, PrismaValue)) -> Self {
        WriteOperation::composite_set(pv)
    }
}

impl From<(&SelectedField, PrismaValue)> for WriteOperation {
    fn from((selection, pv): (&SelectedField, PrismaValue)) -> Self {
        match selection {
            SelectedField::Scalar(sf) => (sf, pv).into(),
            SelectedField::Composite(cs) => (&cs.field, pv).into(),
        }
    }
}

impl TryInto<PrismaValue> for WriteOperation {
    type Error = ConnectorError;

    fn try_into(self) -> Result<PrismaValue, Self::Error> {
        match self {
            WriteOperation::Scalar(ScalarWriteOperation::Set(pv)) => Ok(pv),
            WriteOperation::Composite(CompositeWriteOperation::Set(pv)) => Ok(pv),
            x => Err(ConnectorError::from_kind(ErrorKind::InternalConversionError(format!(
                "Unable to convert write expression {:?} into prisma value.",
                x
            )))),
        }
    }
}

impl From<IndexMap<DatasourceFieldName, WriteOperation>> for WriteArgs {
    fn from(args: IndexMap<DatasourceFieldName, WriteOperation>) -> Self {
        Self { args }
    }
}

impl From<Vec<(DatasourceFieldName, WriteOperation)>> for WriteArgs {
    fn from(pairs: Vec<(DatasourceFieldName, WriteOperation)>) -> Self {
        Self {
            args: pairs.into_iter().collect(),
        }
    }
}

impl WriteArgs {
    pub fn new() -> Self {
        Self { args: IndexMap::new() }
    }

    pub fn insert<T, V>(&mut self, key: T, arg: V)
    where
        T: Into<DatasourceFieldName>,
        V: Into<WriteOperation>,
    {
        self.args.insert(key.into(), arg.into());
    }

    pub fn has_arg_for(&self, field: &str) -> bool {
        self.args.contains_key(field)
    }

    pub fn get_field_value(&self, field: &str) -> Option<&WriteOperation> {
        self.args.get(field)
    }

    pub fn take_field_value(&mut self, field: &str) -> Option<WriteOperation> {
        self.args.remove(field)
    }

    pub fn keys(&self) -> Keys<DatasourceFieldName, WriteOperation> {
        self.args.keys()
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn add_datetimes(&mut self, model: &ModelRef) {
        let now = PrismaValue::DateTime(Utc::now().into());
        let updated_at_field = model.fields().updated_at();

        if let Some(f) = updated_at_field {
            if self.args.get(f.db_name()).is_none() {
                self.args.insert(f.into(), WriteOperation::scalar_set(now));
            }
        }
    }

    pub fn update_datetimes(&mut self, model: ModelRef) {
        if !self.args.is_empty() {
            if let Some(field) = model.fields().updated_at() {
                if self.args.get(field.db_name()).is_none() {
                    let now = PrismaValue::DateTime(Utc::now().into());

                    self.args.insert(field.into(), WriteOperation::scalar_set(now));
                }
            }
        }
    }

    pub fn as_record_projection(&self, model_projection: ModelProjection) -> Option<SelectionResult> {
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

/// Picks all arguments out of `args` that are updating a value for a field
/// contained in `projection`, as those need to be merged into the records later on.
pub fn pick_args(projection: &ModelProjection, args: &WriteArgs) -> WriteArgs {
    let pairs: Vec<_> = projection
        .scalar_fields()
        .into_iter()
        .filter_map(|field| {
            args.get_field_value(field.db_name())
                .map(|v| (DatasourceFieldName::from(&field), v.clone()))
        })
        .collect();

    WriteArgs::from(pairs)
}

/// Merges the incoming write argument values into the given, already loaded, ids. Overwrites existing values.
pub fn merge_write_args(loaded_ids: Vec<SelectionResult>, incoming_args: WriteArgs) -> Vec<SelectionResult> {
    if loaded_ids.is_empty() || incoming_args.is_empty() {
        return loaded_ids;
    }

    // Contains all positions that need to be updated with the given expression.
    let positions: IndexMap<usize, &WriteOperation> = loaded_ids
        .first()
        .unwrap()
        .pairs
        .iter()
        .enumerate()
        .filter_map(|(i, (selection, _))| incoming_args.get_field_value(selection.db_name()).map(|val| (i, val)))
        .collect();

    loaded_ids
        .into_iter()
        .map(|mut id| {
            for (position, write_op) in positions.iter() {
                let current_val = id.pairs[position.to_owned()].1.clone();
                id.pairs[position.to_owned()].1 =
                    apply_expression(current_val, (*write_op.as_scalar().unwrap()).clone());
            }

            id
        })
        .collect()
}

pub fn apply_expression(val: PrismaValue, scalar_write: ScalarWriteOperation) -> PrismaValue {
    match scalar_write {
        ScalarWriteOperation::Field(_) => unimplemented!(),
        ScalarWriteOperation::Set(pv) => pv,
        ScalarWriteOperation::Add(rhs) => val + rhs,
        ScalarWriteOperation::Substract(rhs) => val - rhs,
        ScalarWriteOperation::Multiply(rhs) => val * rhs,
        ScalarWriteOperation::Divide(rhs) => val / rhs,
    }
}
