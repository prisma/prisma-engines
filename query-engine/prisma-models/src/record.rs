use crate::{DomainError, FieldSelection, ModelProjection, OrderBy, PrismaValue, SelectionResult, SortOrder};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SingleRecord {
    pub record: Record,
    pub field_names: Vec<String>,
}

impl From<SingleRecord> for ManyRecords {
    fn from(single: SingleRecord) -> ManyRecords {
        ManyRecords {
            records: vec![single.record],
            field_names: single.field_names,
        }
    }
}

impl SingleRecord {
    pub fn new(record: Record, field_names: Vec<String>) -> Self {
        Self { record, field_names }
    }

    pub fn extract_selection_result(&self, extraction_selection: &FieldSelection) -> crate::Result<SelectionResult> {
        self.record
            .extract_selection_result(&self.field_names, extraction_selection)
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

    pub fn empty(selected_fields: &FieldSelection) -> Self {
        Self {
            records: Vec::new(),
            field_names: selected_fields.prisma_names().collect(),
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
            let mut orderings = order_bys.iter().map(|o| match o {
                OrderBy::Scalar(by_scalar) => {
                    let index = field_indices[by_scalar.field.db_name()];

                    match by_scalar.sort_order {
                        SortOrder::Ascending => a.values[index].cmp(&b.values[index]),
                        SortOrder::Descending => b.values[index].cmp(&a.values[index]),
                    }
                }
                OrderBy::ScalarAggregation(_) => unimplemented!(),
                OrderBy::ToManyAggregation(_) => unimplemented!(),
                OrderBy::Relevance(_) => unimplemented!(),
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

    /// Builds `SelectionResults` from this `ManyRecords` based on the given FieldSelection.
    pub fn extract_selection_results(&self, selections: &FieldSelection) -> crate::Result<Vec<SelectionResult>> {
        self.records
            .iter()
            .map(|record| record.extract_selection_result(&self.field_names, selections))
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

    /// Deduplicate the wrapped records
    pub fn with_unique_records(mut self) -> Self {
        self.records = self.records.into_iter().unique().collect();
        self
    }
}

impl From<(Vec<Vec<PrismaValue>>, &FieldSelection)> for ManyRecords {
    fn from((values, selected_fields): (Vec<Vec<PrismaValue>>, &FieldSelection)) -> Self {
        Self {
            records: values
                .into_iter()
                .map(|value| Record {
                    values: value,
                    parent_id: None,
                })
                .collect(),
            field_names: selected_fields.db_names().collect(),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Record {
    pub values: Vec<PrismaValue>,
    pub parent_id: Option<SelectionResult>,
}

impl Record {
    pub fn new(values: Vec<PrismaValue>) -> Record {
        Record {
            values,
            ..Default::default()
        }
    }

    /// Extract a `SelectionResult` from this `Record`
    /// `field_names`: Database names of the fields contained in this `Record`.
    /// `selected_fields`: The selection to extract.
    pub fn extract_selection_result(
        &self,
        field_names: &[String],
        extraction_selection: &FieldSelection,
    ) -> crate::Result<SelectionResult> {
        let pairs: Vec<_> = extraction_selection
            .selections()
            .into_iter()
            .map(|selection| {
                self.get_field_value(field_names, selection.db_name())
                    .and_then(|val| Ok((selection.clone(), selection.coerce_value(val.clone())?)))
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(SelectionResult::new(pairs))
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
                    .map(|source_field| self.get_field_value(field_names, source_field.name()))
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(x)
    }

    pub fn get_field_value(&self, field_names: &[String], field: &str) -> crate::Result<&PrismaValue> {
        let index = field_names.iter().position(|r| r == field).map(Ok).unwrap_or_else(|| {
            Err(DomainError::FieldNotFound {
                name: field.to_string(),
                container_type: "field",
                container_name: format!("Record values: {:?}. Field names: {:?}.", &self, &field_names),
            })
        })?;

        Ok(&self.values[index])
    }

    pub fn set_parent_id(&mut self, parent_id: SelectionResult) {
        self.parent_id = Some(parent_id);
    }
}
