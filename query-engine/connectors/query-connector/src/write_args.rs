use crate::{
    error::{ConnectorError, ErrorKind},
    Filter,
};
use indexmap::{map::Keys, IndexMap};
use prisma_models::{
    CompositeFieldRef, Field, ModelProjection, ModelRef, PrismaValue, ScalarFieldRef, SelectedField, SelectionResult,
};
use std::{borrow::Borrow, convert::TryInto, ops::Deref};

/// WriteArgs represent data to be written to an underlying data source.
#[derive(Debug, PartialEq, Clone)]
pub struct WriteArgs {
    pub request_now: PrismaValue,
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

    pub fn scalar_unset(should_unset: bool) -> Self {
        Self::Scalar(ScalarWriteOperation::Unset(should_unset))
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

    pub fn composite_unset(should_unset: bool) -> Self {
        Self::Composite(CompositeWriteOperation::Unset(should_unset))
    }

    pub fn composite_update(writes: Vec<(DatasourceFieldName, WriteOperation)>) -> Self {
        Self::Composite(CompositeWriteOperation::Update(NestedWrite { writes }))
    }

    pub fn composite_push(pv: PrismaValue) -> Self {
        Self::Composite(CompositeWriteOperation::Push(pv))
    }

    pub fn composite_upsert(set: CompositeWriteOperation, update: CompositeWriteOperation) -> Self {
        Self::Composite(CompositeWriteOperation::Upsert {
            set: Box::new(set),
            update: Box::new(update),
        })
    }

    pub fn composite_update_many(filter: Filter, update: CompositeWriteOperation) -> Self {
        Self::Composite(CompositeWriteOperation::UpdateMany {
            filter,
            update: Box::new(update),
        })
    }

    pub fn composite_delete_many(filter: Filter) -> Self {
        Self::Composite(CompositeWriteOperation::DeleteMany { filter })
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

    pub fn try_into_scalar(self) -> Option<ScalarWriteOperation> {
        if let Self::Scalar(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn try_into_composite(self) -> Option<CompositeWriteOperation> {
        if let Self::Composite(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ScalarWriteOperation {
    /// Reference to another field on the same model (unused at the moment).
    Field(DatasourceFieldName),

    /// Write plain value to field.
    Set(PrismaValue),

    /// Unsets a field (only for MongoDB for now)
    Unset(bool),

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
    Push(PrismaValue),
    Unset(bool),
    Update(NestedWrite),
    Upsert {
        set: Box<CompositeWriteOperation>,
        update: Box<CompositeWriteOperation>,
    },
    UpdateMany {
        filter: Filter,
        update: Box<CompositeWriteOperation>,
    },
    DeleteMany {
        filter: Filter,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub struct NestedWrite {
    pub writes: Vec<(DatasourceFieldName, WriteOperation)>,
}

#[derive(Debug, Clone, Default)]
pub struct FieldPath {
    pub alias: Option<String>,
    pub path: Vec<String>,
}

impl FieldPath {
    pub fn new_from_segment(field: &Field) -> Self {
        let mut path = Self::default();
        path.add_segment(field);

        path
    }

    pub fn new_from_alias(alias: impl Into<String>) -> Self {
        Self {
            alias: Some(alias.into()),
            path: vec![],
        }
    }

    pub fn add_segment(&mut self, field: &Field) {
        self.path.push(field.db_name().to_owned());
    }

    /// Keep only the last element of the path
    pub fn keep_last(&mut self) {
        self.path.drain(0..self.path.len() - 1);
    }

    pub fn take(&mut self, n: usize) {
        self.path = self.path[0..n].to_vec();
    }

    pub fn path(&self, include_alias: bool) -> String {
        let rendered_path = self.path.join(".");

        if !include_alias {
            return rendered_path;
        }

        if let Some(alias) = &self.alias {
            if self.path.is_empty() {
                alias.to_owned()
            } else {
                format!("${alias}.{rendered_path}")
            }
        } else {
            rendered_path
        }
    }

    pub fn dollar_path(&self, include_alias: bool) -> String {
        format!("${}", self.path(include_alias))
    }

    pub fn identifier(&self) -> String {
        let rendered_path = self.path.join("_");

        if let Some(alias) = &self.alias {
            if self.path.is_empty() {
                alias.to_owned()
            } else {
                format!("{alias}_{rendered_path}")
            }
        } else {
            rendered_path
        }
    }
}

impl NestedWrite {
    /// Unfolds nested writes into a flat list of `WriteOperation`s.
    ///
    /// Given the following `NestedWrite`:
    /// ```text
    /// Vec [(
    ///       "field_a",
    ///       WriteOperation::Composite(Update(Vec[("field_b", WriteOperation::Composite(Set("3")))]))
    ///     )]
    /// ```
    /// `unfold` will roughly return:
    /// ```text
    /// Vec[(Set("3"), Field("field_b"), "field_a.field_b")]
    /// ```
    /// where:
    ///  - `Set("3")` is the write operation to execute
    ///  - `Field("field_b")` is the field on which to execute the write operation
    /// - `"field_a.field_b"` is the path for MongoDB to access the nested field
    pub fn unfold(self, field: &Field, field_path: FieldPath) -> Vec<(WriteOperation, Field, FieldPath)> {
        self.unfold_internal(field.clone(), field_path)
    }

    fn unfold_internal(self, field: Field, field_path: FieldPath) -> Vec<(WriteOperation, Field, FieldPath)> {
        let mut nested_writes: Vec<(WriteOperation, Field, FieldPath)> = vec![];

        for (DatasourceFieldName(db_name), write) in self.writes {
            let nested_ct = field.as_composite().unwrap().typ();
            let nested_field = nested_ct.find_field_by_db_name(&db_name).unwrap();
            let mut new_path = field_path.clone();

            new_path.add_segment(&nested_field);

            match write {
                WriteOperation::Composite(CompositeWriteOperation::Update(nested_write)) => {
                    nested_writes.extend(nested_write.unfold_internal(nested_field.clone(), new_path));
                }
                _ => {
                    nested_writes.push((write, nested_field.clone(), new_path));
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
                "Unable to convert write expression {x:?} into prisma value."
            )))),
        }
    }
}

impl WriteArgs {
    pub fn new(args: IndexMap<DatasourceFieldName, WriteOperation>, request_now: PrismaValue) -> Self {
        Self { args, request_now }
    }

    pub fn new_empty(request_now: PrismaValue) -> Self {
        Self {
            args: Default::default(),
            request_now,
        }
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

    // @updatedAt
    pub fn add_datetimes(&mut self, model: &ModelRef) {
        let updated_at_fields = model.fields().updated_at();
        let value = &self.request_now;

        for f in updated_at_fields {
            if self.args.get(f.db_name()).is_none() {
                self.args.insert((&f).into(), WriteOperation::scalar_set(value.clone()));
            }
        }
    }

    pub fn update_datetimes(&mut self, model: &ModelRef) {
        if !self.args.is_empty() {
            self.add_datetimes(model)
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
    let pairs = projection
        .scalar_fields()
        .into_iter()
        .filter_map(|field| {
            args.get_field_value(field.db_name())
                .map(|v| (DatasourceFieldName::from(&field), v.clone()))
        })
        .collect();

    WriteArgs::new(pairs, args.request_now.clone())
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
        ScalarWriteOperation::Unset(_) => unimplemented!(),
    }
}
