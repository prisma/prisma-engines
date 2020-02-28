use crate::{DataSourceFieldRef, DomainError, ModelIdentifier, PrismaValue, RecordIdentifier};

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

    pub fn identifier(&self, id: &ModelIdentifier) -> crate::Result<RecordIdentifier> {
        self.record.identifier(&self.field_names, id)
    }

    pub fn get_field_value(&self, field: &str) -> crate::Result<&PrismaValue> {
        self.record.get_field_value(&self.field_names, field)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ManyRecords {
    pub records: Vec<Record>,
    pub field_names: Vec<String>,
}

impl ManyRecords {
    pub fn identifiers(&self, model_id: &ModelIdentifier) -> crate::Result<Vec<RecordIdentifier>> {
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

    pub fn identifier(&self, field_names: &[String], id: &ModelIdentifier) -> crate::Result<RecordIdentifier> {
        let pairs: Vec<(DataSourceFieldRef, PrismaValue)> = id
            .fields()
            .into_iter()
            .flat_map(|id_field| {
                let source_fields = id_field.data_source_fields();

                source_fields.into_iter().map(|source_field| {
                    self.get_field_value(field_names, &source_field.name)
                        .map(|val| (source_field, val.clone()))
                })
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(RecordIdentifier { pairs })
    }

    pub fn identifying_values(&self, field_names: &[String], id: &ModelIdentifier) -> crate::Result<Vec<&PrismaValue>> {
        let x: Vec<&PrismaValue> = id
            .fields()
            .into_iter()
            .flat_map(|id_field| {
                let source_fields = id_field.data_source_fields();

                source_fields
                    .into_iter()
                    .map(|source_field| self.get_field_value(field_names, &source_field.name))
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(x)
    }

    pub fn get_field_value(&self, field_names: &[String], field: &str) -> crate::Result<&PrismaValue> {
        let index = field_names.iter().position(|r| r == field).map(Ok).unwrap_or_else(|| {
            Err(DomainError::FieldNotFound {
                name: field.to_string(),
                model: format!(
                    "Field not found in record {:?}. Field names are: {:?}, looking for: {:?}",
                    &self, &field_names, field
                ),
            })
        })?;

        // [DTODO] Revert to old code
        // Ok(&self.values[index])
        match self.values.get(index) {
            Some(v) => Ok(v),
            None => Err(DomainError::FieldNotFound {
                name: field.to_owned(),
                model: format!(
                    "Field not found in record {:?}. Field names are: {:?}, looking for: {:?}",
                    &self, &field_names, field
                ),
            }),
        }
    }

    pub fn set_parent_id(&mut self, parent_id: RecordIdentifier) {
        self.parent_id = Some(parent_id);
    }
}
