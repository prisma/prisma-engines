use crate::{DomainError as Error, DomainResult, Field, PrismaValue};

// Collection of fields of which the primary identifier of a model is composed of.
pub type PrimaryIdentifier = Vec<Field>;

// Collection of field to value pairs corresponding to the
// WIP: Holding an arc here is a terrible idea. After seeing what the final code
//      looks like, we need to revise that decision.
#[derive(Debug, Clone)]
pub struct RecordIdentifier {
    pub pairs: Vec<(Field, PrismaValue)>,
}

impl RecordIdentifier {
    pub fn new(pairs: Vec<(Field, PrismaValue)>) -> Self {
        Self { pairs }
    }

    pub fn add(&mut self, pair: (Field, PrismaValue)) {
        self.pairs.push(pair);
    }
}

impl Into<RecordIdentifier> for (Field, PrismaValue) {
    fn into(self) -> RecordIdentifier {
        RecordIdentifier::new(vec![self])
    }
}

impl Into<RecordIdentifier> for Vec<(Field, PrismaValue)> {
    fn into(self) -> RecordIdentifier {
        RecordIdentifier::new(self)
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

    pub fn primary_id(&self, id: &PrimaryIdentifier) -> DomainResult<RecordIdentifier> {
        self.record.primary_id(&self.field_names, id)
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
    pub fn primary_ids(&self, id: PrimaryIdentifier) -> DomainResult<Vec<RecordIdentifier>> {
        self.records
            .iter()
            .map(|record| record.primary_id(&self.field_names, &id).map(|i| i.clone()))
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

    pub fn primary_id(&self, field_names: &[String], id: &PrimaryIdentifier) -> DomainResult<RecordIdentifier> {
        let pairs: Vec<(Field, PrismaValue)> = id
            .into_iter()
            .map(|id_field| {
                self.get_field_value(field_names, id_field.name())
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
