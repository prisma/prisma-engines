use query_structure::{FieldSelection, Filter, Model, RecordFilter, ScalarFieldRef, WriteArgs};

#[derive(Debug, Clone)]
pub struct NativeUpsert {
    name: String,
    model: Model,
    record_filter: RecordFilter,
    create: WriteArgs,
    update: WriteArgs,
    conflict_target: Vec<ScalarFieldRef>,
    pub selected_fields: FieldSelection,
    pub selection_order: Vec<String>,
}

impl NativeUpsert {
    /// `conflict_target` is the non-empty column list the statement arbitrates on:
    /// the columns of the very constraint the `where` clause named, as resolved by
    /// the query graph builder. It is taken rather than recomputed here so that the
    /// check deciding this query is buildable and the columns it emits cannot drift
    /// apart; an empty list renders as `ON CONFLICT ()`, a syntax error rather than
    /// a planning failure.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        model: Model,
        record_filter: RecordFilter,
        create: WriteArgs,
        update: WriteArgs,
        conflict_target: Vec<ScalarFieldRef>,
        selected_fields: FieldSelection,
        selection_order: Vec<String>,
    ) -> Self {
        Self {
            name,
            model,
            record_filter,
            create,
            update,
            conflict_target,
            selected_fields,
            selection_order,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn update(&self) -> &WriteArgs {
        &self.update
    }

    pub fn update_mut(&mut self) -> &mut WriteArgs {
        &mut self.update
    }

    pub fn create(&self) -> &WriteArgs {
        &self.create
    }

    pub fn create_mut(&mut self) -> &mut WriteArgs {
        &mut self.create
    }

    pub fn conflict_target(&self) -> &[ScalarFieldRef] {
        &self.conflict_target
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
