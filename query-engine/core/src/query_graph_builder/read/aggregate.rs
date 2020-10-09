use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, ReadQuery};
use connector::Aggregator;
use prisma_models::{ModelRef, ScalarFieldRef};

pub struct AggregateRecordsBuilder {
    field: ParsedField,
    model: ModelRef,
}

impl AggregateRecordsBuilder {
    pub fn new(field: ParsedField, model: ModelRef) -> Self {
        Self { field, model }
    }

    /// Resolves the given field as a aggregation query.
    fn resolve_query(field: ParsedField, model: &ModelRef) -> QueryGraphBuilderResult<Aggregator> {
        let query = match field.name.as_str() {
            "count" => Aggregator::Count,
            "avg" => Aggregator::Average(Self::resolve_fields(model, field)),
            "sum" => Aggregator::Sum(Self::resolve_fields(model, field)),
            "min" => Aggregator::Min(Self::resolve_fields(model, field)),
            "max" => Aggregator::Max(Self::resolve_fields(model, field)),
            _ => unreachable!(),
        };

        Ok(query)
    }

    fn resolve_fields(model: &ModelRef, field: ParsedField) -> Vec<ScalarFieldRef> {
        let fields = field.nested_fields.unwrap().fields;
        let scalars = model.fields().scalar();

        fields
            .into_iter()
            .map(|f| {
                scalars
                    .iter()
                    .find_map(|sf| if sf.name == f.name { Some(sf.clone()) } else { None })
                    .expect("Expected validation to guarantee valid aggregation fields.")
            })
            .collect()
    }

    fn collect_selection_tree(fields: &[ParsedField]) -> Vec<(String, Option<Vec<String>>)> {
        fields
            .iter()
            .map(|field| {
                (
                    field.name.clone(),
                    field
                        .nested_fields
                        .as_ref()
                        .map(|nested_object| nested_object.fields.iter().map(|f| f.name.clone()).collect()),
                )
            })
            .collect()
    }
}

impl Builder<ReadQuery> for AggregateRecordsBuilder {
    fn build(self) -> QueryGraphBuilderResult<ReadQuery> {
        let name = self.field.name;
        let alias = self.field.alias;
        let model = self.model;
        let nested_fields = self.field.nested_fields.unwrap().fields;
        let selection_order = Self::collect_selection_tree(&nested_fields);
        let args = extractors::extract_query_args(self.field.arguments, &model)?;

        // Reject unstable cursors for aggregations, because we can't do post-processing on those (we haven't implemented a in-memory aggregator yet).
        if args.contains_unstable_cursor() {
            return Err(QueryGraphBuilderError::InputError(
                "The chosen cursor and orderBy combination is not stable (unique) and can't be used for aggregations."
                    .to_owned(),
            ));
        }

        let aggregators: Vec<_> = nested_fields
            .into_iter()
            .map(|field| Self::resolve_query(field, &model))
            .collect::<QueryGraphBuilderResult<_>>()?;

        Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
            name,
            alias,
            model,
            selection_order,
            args,
            aggregators,
        }))
    }
}
