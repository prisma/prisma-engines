use crate::{DomainError as Error, DomainResult, PrismaValue, ScalarFieldRef};

// Collection of fields of which the primary identifier of a model is composed of.
// Todo: Currently, this uses arcs, which is not ideal, but also not terrible compared
// Arcs in the RecordIdentifier.
#[derive(Debug, Clone, Default)]
pub struct ModelIdentifier {
    fields: Vec<ScalarFieldRef>,
}

impl From<ScalarFieldRef> for ModelIdentifier {
    fn from(f: ScalarFieldRef) -> Self {
        Self { fields: vec![f] }
    }
}

impl ModelIdentifier {
    pub fn new(fields: Vec<ScalarFieldRef>) -> Self {
        Self { fields }
    }

    pub fn names<'a>(&'a self) -> impl Iterator<Item = &'a str> + 'a {
        self.fields.iter().map(|field| field.name.as_str())
    }

    pub fn fields<'a>(&'a self) -> impl Iterator<Item = &'a ScalarFieldRef> + 'a {
        self.fields.iter()
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_singular_field(&self) -> bool {
        self.len() == 1
    }
}

impl IntoIterator for ModelIdentifier {
    type Item = ScalarFieldRef;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

// Collection of field to value pairs corresponding to the ModelIdentifier the record belongs to.
// Todo: Storing Arcs is not a great idea, as practically every single record produced by a query
// essentially clones the arcs of the model identifier. After the main work on multi/any-id-fields
// is done. Maybe references are acceptable to use here.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordIdentifier {
    pub pairs: Vec<(ScalarFieldRef, PrismaValue)>,
}

impl RecordIdentifier {
    pub fn new(pairs: Vec<(ScalarFieldRef, PrismaValue)>) -> Self {
        Self { pairs }
    }

    pub fn add(&mut self, pair: (ScalarFieldRef, PrismaValue)) {
        self.pairs.push(pair);
    }

    pub fn values(&self) -> impl Iterator<Item = PrismaValue> + '_ {
        self.pairs.iter().map(|p| p.1.clone())
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl IntoIterator for RecordIdentifier {
    type Item = (ScalarFieldRef, PrismaValue);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.pairs.into_iter()
    }
}

impl From<(ScalarFieldRef, PrismaValue)> for RecordIdentifier {
    fn from(tup: (ScalarFieldRef, PrismaValue)) -> Self {
        Self::new(vec![tup])
    }
}

impl From<Vec<(ScalarFieldRef, PrismaValue)>> for RecordIdentifier {
    fn from(tup: Vec<(ScalarFieldRef, PrismaValue)>) -> Self {
        Self::new(tup)
    }
}

#[derive(Debug, Clone)]
pub struct SingleRecord {
    pub record: Record,
    pub field_names: Vec<String>,
}

impl Into<ManyRecords> for SingleRecord {
    fn into(self) -> ManyRecords {
        ManyRecords {
            records: vec![self.record],
            field_names: self.field_names,
        }
    }
}

impl SingleRecord {
    pub fn new(record: Record, field_names: Vec<String>) -> Self {
        Self { record, field_names }
    }

    pub fn identifier(&self, id: &ModelIdentifier) -> DomainResult<RecordIdentifier> {
        self.record.identifier(&self.field_names, id)
    }

    pub fn get_field_value(&self, field: &str) -> DomainResult<&PrismaValue> {
        self.record.get_field_value(&self.field_names, field)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ManyRecords {
    pub records: Vec<Record>,
    pub field_names: Vec<String>,
}

impl ManyRecords {
    pub fn identifiers(&self, model_id: &ModelIdentifier) -> DomainResult<Vec<RecordIdentifier>> {
        self.records
            .iter()
            .map(|record| record.identifier(&self.field_names, model_id).map(|i| i.clone()))
            .collect()
    }

    /// Maps into a Vector of (field_name, value) tuples
    pub fn as_pairs(&self) -> Vec<Vec<(String, PrismaValue)>> {
        self.records
            .iter()
            .map(|record| {
                record
                    .values
                    .iter()
                    .zip(self.field_names.iter())
                    .map(|(value, name)| (name.clone(), value.clone()))
                    .collect()
            })
            .collect()
    }

    /// Reverses the wrapped records in place
    pub fn reverse(&mut self) {
        self.records.reverse();
    }
}

#[derive(Debug, Default, Clone)]
pub struct Record {
    pub values: Vec<PrismaValue>,
    pub parent_id: Option<RecordIdentifier>,
}

impl Record {
    pub fn new(values: Vec<PrismaValue>) -> Record {
        Record {
            values,
            ..Default::default()
        }
    }

    pub fn identifier(&self, field_names: &[String], id: &ModelIdentifier) -> DomainResult<RecordIdentifier> {
        let pairs: Vec<(ScalarFieldRef, PrismaValue)> = id
            .fields()
            .into_iter()
            .map(|id_field| {
                self.get_field_value(field_names, &id_field.name)
                    .map(|val| (id_field.clone(), val.clone()))
            })
            .collect::<DomainResult<Vec<_>>>()?;

        Ok(RecordIdentifier { pairs })
    }

    pub fn get_field_value(&self, field_names: &[String], field: &str) -> DomainResult<&PrismaValue> {
        let index = field_names.iter().position(|r| r == field).map(Ok).unwrap_or_else(|| {
            Err(Error::FieldNotFound {
                name: field.to_string(),
                model: String::new(),
            })
        })?;

        Ok(&self.values[index])
    }

    pub fn set_parent_id(&mut self, parent_id: RecordIdentifier) {
        self.parent_id = Some(parent_id);
    }
}
