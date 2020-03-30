use super::*;
use crate::{query_document::ParsedField, AggregateRecordsQuery, AggregationQuery, ReadQuery};
use prisma_models::ModelRef;

pub struct AggregateRecordsBuilder {
    field: ParsedField,
    model: ModelRef,
}

impl AggregateRecordsBuilder {
    pub fn new(field: ParsedField, model: ModelRef) -> Self {
        Self { field, model }
    }

    fn resolve_query(field: ParsedField, model: &ModelRef) -> QueryGraphBuilderResult<AggregationQuery> {
        let query = match field.name {
            name if &name == "count" => {
                AggregationQuery::Count(name, extractors::extract_query_args(field.arguments, model)?)
            }
            _ => unreachable!(),
        };

        Ok(query)
    }
}

impl Builder<ReadQuery> for AggregateRecordsBuilder {
    fn build(self) -> QueryGraphBuilderResult<ReadQuery> {
        let name = self.field.name;
        let alias = self.field.alias;
        let model = self.model;
        let nested_fields = self.field.nested_fields.unwrap().fields;
        let selection_order: Vec<String> = collect_selection_order(&nested_fields);

        let queries: Vec<_> = nested_fields
            .into_iter()
            .map(|field| Self::resolve_query(field, &model))
            .collect::<QueryGraphBuilderResult<_>>()?;

        Ok(ReadQuery::AggregateRecordsQuery(AggregateRecordsQuery {
            name,
            alias,
            model,
            selection_order,
            queries,
        }))
    }
}
