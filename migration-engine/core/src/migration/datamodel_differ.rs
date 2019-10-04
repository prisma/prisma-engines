#[deny(rust_2018_idioms)]
use datamodel::ast;

#[derive(Default)]
pub(crate) struct DatamodelDiffer;

enum DatamodelChange {}

#[derive(Default)]
pub(crate) struct DatamodelDiff {
    changes: Vec<DatamodelChange>,
}

impl DatamodelDiff {
    fn push_change(&mut self, change: DatamodelChange) {
        self.changes.push(change)
    }
}

pub(crate) struct SchemaPair<'a> {
    previous: &'a ast::Datamodel,
    next: &'a ast::Datamodel,
}

impl<'a> SchemaPair<'a> {
    /// Iterate over the top-level elements of both old and new schema, in the order
    fn top_pairs(&self) -> impl Iterator<Item = (Option<&ast::Top>, Option<&ast::Top>)> {
        let existing_models = self.previous.models.iter().map(move |previous_model| {
            let next = self.next.models.iter().find(|next_model| {
                // We need to compare the type of top-level element too, to avoid matching an enum with a model of the same name, for example.
                next_model.name() == previous_model.name() && previous_model.get_type() == next_model.get_type()
            });
            (Some(previous_model), next)
        });
        let new_models = self
            .next
            .models
            .iter()
            .filter(move |next_model| {
                self.previous
                    .models
                    .iter()
                    .find(|previous_model| {
                        previous_model.name() == next_model.name() && previous_model.get_type() == next_model.get_type()
                    })
                    .is_none()
            })
            .map(|next_model| (None, Some(next_model)));

        existing_models.chain(new_models)
    }

    /// Iterate over the all the models, by matching pairs.
    fn model_pairs(&self) -> impl Iterator<Item = (Option<&ast::Model>, Option<&ast::Model>)> {
        self.top_pairs()
            .map(|(previous_top, next_top)| {
                (
                    previous_top.and_then(ast::Top::as_model),
                    next_top.and_then(ast::Top::as_model),
                )
            })
            .filter(|(a, b)| a.is_some() || b.is_some())
    }
}

impl DatamodelDiffer {
    pub(crate) fn diff(&mut self, schemas: &SchemaPair<'_>) {
        self.visit_top(&schemas);
    }

    fn visit_top(&mut self, schemas: &SchemaPair<'_>) {
        schemas.model_pairs().for_each(|pair| self.visit_models(pair))
    }

    fn visit_models(&mut self, model_pair: (Option<&ast::Model>, Option<&ast::Model>)) {
        match model_pair {
            (Some(previous_model), Some(next_model)) => {
                field_pairs(previous_model, next_model).for_each(|pair| self.visit_fields(pair))
            }
            (Some(previous_model), None) => self.visit_dropped_model(previous_model),
            (None, Some(next_model)) => self.visit_created_model(next_model),
            (None, None) => unreachable!("empty model pair"),
        }
    }

    fn visit_fields(&mut self, field_pair: (Option<&ast::Field>, Option<&ast::Field>)) {
        unimplemented!()
    }

    fn visit_dropped_model(&mut self, dropped_model: &ast::Model) {
        unimplemented!()
    }

    fn visit_created_model(&mut self, created_model: &ast::Model) {
        unimplemented!()
    }
}

fn field_pairs<'a>(
    previous_model: &'a ast::Model,
    next_model: &'a ast::Model,
) -> impl Iterator<Item = (Option<&'a ast::Field>, Option<&'a ast::Field>)> {
    let existing_fields = previous_model.fields.iter().map(move |previous_field| {
        let next = next_model
            .fields
            .iter()
            .find(|next_field| previous_field.name.name == next_field.name.name);
        (Some(previous_field), next)
    });

    let new_fields = next_model
        .fields
        .iter()
        .filter(move |next_field| {
            previous_model
                .fields
                .iter()
                .find(|previous_field| previous_field.name.name == next_field.name.name)
                .is_none()
        })
        .map(|next_field| (None, Some(next_field)));

    existing_fields.chain(new_fields)
}
