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

    pub fn unique_constraint(&self) -> Vec<ScalarFieldRef> {
        let compound_indexes = self.model.unique_indexes();
        let scalars = self.record_filter.filter.scalars();
        let unique_index = compound_indexes.into_iter().find(|index| {
            let index_fields = index.fields();
            index_fields.into_iter().all(|f| scalars.contains(&f))
        });

        if let Some(index) = unique_index {
            return index.fields();
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
}

impl std::fmt::Display for NativeUpsert {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Upsert(model: {}, filter: {:?}, create: {:?}, update: {:?}",
            self.model.name, self.record_filter, self.create, self.update
        )
    }
}
