use crate::{DomainError, ModelProjection, OrderBy, PrismaValue, RecordProjection, ScalarFieldRef, SortOrder};
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

    pub fn empty(selected_fields: &ModelProjection) -> Self {
        Self {
            records: Vec::new(),
            field_names: selected_fields.names().map(|n| n.to_string()).collect(),
        }
    }

    pub fn from_projection(projection: Vec<Vec<PrismaValue>>, selected_fields: &ModelProjection) -> Self {
        Self {
            records: projection
                .into_iter()
                .map(|v| Record {
                    values: v,
                    parent_id: None,
                })
                .collect(),
            field_names: selected_fields.db_names().collect(),
        }
    }

    pub fn order_by(&mut self, order_bys: &[OrderBy]) {
        let field_indices: HashMap<&str, usize> = self
            .field_names
            .iter()
            .enumerate()
            .map(|(i, name)| (name.as_str(), i))
            .collect();

        self.records.sort_by(|a, b| {
            let mut orderings = order_bys.iter().map(|o| {
                let index = field_indices[o.field.db_name()];
                match o.sort_order {
                    SortOrder::Ascending => a.values[index].cmp(&b.values[index]),
                    SortOrder::Descending => b.values[index].cmp(&a.values[index]),
                }
            });

            orderings
                .next()
                .map(|first| orderings.fold(first, |acc, ord| acc.then(ord)))
                .unwrap()
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
        let pairs: Vec<(ScalarFieldRef, PrismaValue)> = model_projection
            .fields()
            .into_iter()
            .flat_map(|field| {
                field.scalar_fields().into_iter().map(|field| {
                    self.get_field_value(field_names, field.db_name())
                        .map(|val| (field, val.clone()))
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
                field
                    .scalar_fields()
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
