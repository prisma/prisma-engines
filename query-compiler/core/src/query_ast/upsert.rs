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
    /// `conflict_target` is the non-empty column list the statement arbitrates on,
    /// as chosen by [`conflict_target`]. It is taken rather than recomputed here so
    /// that the check deciding this query is buildable and the columns it emits
    /// cannot drift apart: an empty list renders as `ON CONFLICT ()`, which is a
    /// syntax error rather than a planning failure.
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

/// The columns an `INSERT ... ON CONFLICT` can name as its conflict target for
/// `filter`, or `None` when the model carries no constraint the database could
/// infer from them.
///
/// Partial (`WHERE`-filtered) unique indexes are never candidates. Inferring one
/// requires repeating its predicate in the statement, which the generated SQL
/// does not carry, so PostgreSQL rejects it with 42P10 ("there is no unique or
/// exclusion constraint matching the ON CONFLICT specification") on every call,
/// whatever the data. `None` therefore means "the connector-native upsert is not
/// usable here"; callers fall back to the read-then-write graph, which handles
/// those models correctly.
pub fn conflict_target(model: &Model, filter: &Filter) -> Option<Vec<ScalarFieldRef>> {
    let scalars = filter.scalars();

    let unique_index = model
        .unique_indexes()
        .filter(|index| !index.is_partial())
        .find(|index| {
            index
                .fields()
                .all(|f| scalars.contains(&ScalarFieldRef::from((model.dm.clone(), f))))
        });

    if let Some(index) = unique_index {
        return Some(
            index
                .fields()
                .map(|f| ScalarFieldRef::from((model.dm.clone(), f)))
                .collect(),
        );
    }

    // The primary key, which is never partial. This also covers the single-field
    // case that used to fall through to `Filter::unique_scalars`, whose notion of
    // uniqueness counts partial indexes too.
    let ids: Vec<ScalarFieldRef> = model.fields().id_fields()?.collect();
    ids.iter().all(|f| scalars.contains(f)).then_some(ids)
}
