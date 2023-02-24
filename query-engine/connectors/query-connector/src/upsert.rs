use crate::{Filter, RecordFilter, WriteArgs};
use prisma_models::{FieldSelection, ModelRef, ScalarFieldRef};

#[derive(Debug, Clone)]
pub struct NativeUpsert {
    name: String,
    model: ModelRef,
    record_filter: RecordFilter,
    create: WriteArgs,
    update: WriteArgs,
    selected_fields: FieldSelection,
    selection_order: Vec<String>,
}

impl NativeUpsert {
    pub fn new(
        name: String,
        model: ModelRef,
        record_filter: RecordFilter,
        create: WriteArgs,
        update: WriteArgs,
        selected_fields: FieldSelection,
        selection_order: Vec<String>,
    ) -> Self {
        Self {
            name,
            model,
            record_filter,
            create,
            update,
            selected_fields,
            selection_order,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> &ModelRef {
        &self.model
    }

    pub fn update(&self) -> &WriteArgs {
        &self.update
    }

    pub fn create(&self) -> &WriteArgs {
        &self.create
    }

    pub fn unique_constraints(&self) -> Vec<ScalarFieldRef> {
        let compound_indexes = self.model.unique_indexes();
        let scalars = self.record_filter.filter.scalars();
        let unique_index = compound_indexes.into_iter().find(|index| {
            index
                .fields()
                .all(|f| scalars.contains(&ScalarFieldRef::from((self.model.dm.clone(), f))))
        });

        if let Some(index) = unique_index {
            return index
                .fields()
                .map(|f| ScalarFieldRef::from((self.model.dm.clone(), f)))
                .collect();
        }

        if let Some(ids) = self.model.fields().compound_id() {
            if ids.fields().iter().all(|f| scalars.contains(f)) {
                return ids.fields();
            }
        }

        self.record_filter.filter.unique_scalars()
    }

    pub fn filter(&self) -> &Filter {
        &self.record_filter.filter
    }

    pub fn selected_fields(&self) -> &FieldSelection {
        &self.selected_fields
    }

    pub fn selection_order(&self) -> &[String] {
        &self.selection_order
    }

    pub fn record_filter(&self) -> &RecordFilter {
        &self.record_filter
    }
}
