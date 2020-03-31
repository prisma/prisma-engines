use crate::{
    DataSourceFieldRef, DomainError, Field, ModelProjection, OrderBy, PrismaValue, RecordProjection, SortOrder,
};
use std::collections::HashMap;

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

    pub fn projection(&self, projection: &ModelProjection) -> crate::Result<RecordProjection> {
        self.record.projection(&self.field_names, projection)
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
    pub fn new(field_names: Vec<String>) -> Self {
        Self {
            records: Vec::new(),
            field_names,
        }
    }

    pub fn order_by(&mut self, order_by: &OrderBy) {
        let field_indices: HashMap<&str, usize> = self
            .field_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.as_str(), i))
            .collect();

        self.records.sort_by(|a, b| match order_by.field {
            Field::Scalar(ref sf) => {
                let index = field_indices[sf.db_name()];

                match order_by.sort_order {
                    SortOrder::Ascending => a.values[index].cmp(&b.values[index]),
                    SortOrder::Descending => b.values[index].cmp(&a.values[index]),
                }
            }
            Field::Relation(ref rf) => {
                let ds_fields = rf.data_source_fields();
                let mut a_vals = Vec::with_capacity(ds_fields.len());
                let mut b_vals = Vec::with_capacity(ds_fields.len());

                for dsf in ds_fields {
                    let index = field_indices[dsf.name()];
                    a_vals.push(&a.values[index]);
                    b_vals.push(&b.values[index]);
                }

                match order_by.sort_order {
                    SortOrder::Ascending => a_vals.cmp(&b_vals),
                    SortOrder::Descending => b_vals.cmp(&a_vals),
                }
            }
        })
    }

    pub fn push(&mut self, record: Record) {
        self.records.push(record);
    }

    pub fn projections(&self, model_projection: &ModelProjection) -> crate::Result<Vec<RecordProjection>> {
        self.records
            .iter()
            .map(|record| {
                record
                    .projection(&self.field_names, model_projection)
                    .map(|i| i.clone())
            })
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
    pub parent_id: Option<RecordProjection>,
}

impl Record {
    pub fn new(values: Vec<PrismaValue>) -> Record {
        Record {
            values,
            ..Default::default()
        }
    }

    pub fn projection(
        &self,
        field_names: &[String],
        model_projection: &ModelProjection,
    ) -> crate::Result<RecordProjection> {
        let pairs: Vec<(DataSourceFieldRef, PrismaValue)> = model_projection
            .fields()
            .into_iter()
            .flat_map(|field| {
                let source_fields = field.data_source_fields();

                source_fields.into_iter().map(|source_field| {
                    self.get_field_value(field_names, &source_field.name)
                        .map(|val| (source_field, val.clone()))
                })
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(RecordProjection { pairs })
    }

    pub fn identifying_values(
        &self,
        field_names: &[String],
        model_projection: &ModelProjection,
    ) -> crate::Result<Vec<&PrismaValue>> {
        let x: Vec<&PrismaValue> = model_projection
            .fields()
            .into_iter()
            .flat_map(|field| {
                let source_fields = field.data_source_fields();

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

        Ok(&self.values[index])
    }

    pub fn set_parent_id(&mut self, parent_id: RecordProjection) {
        self.parent_id = Some(parent_id);
    }
}
