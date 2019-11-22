use super::*;
use crate::{query_document::ParsedField, ManyRecordsQuery, ReadQuery};
use connector::QueryArguments;
use prisma_models::{ModelRef, RelationFieldRef};

pub struct ReadManyRecordsBuilder {
    field: ParsedField,
    model: ModelRef,
    args: QueryArguments,
    parent_field: Option<RelationFieldRef>,
}

impl ReadManyRecordsBuilder {
    pub fn new(
        field: ParsedField,
        model: ModelRef,
        args: QueryArguments,
        parent_field: Option<RelationFieldRef>,
    ) -> Self {
        Self {
            field,
            model,
            args,
            parent_field,
        }
    }
}

impl Builder<ReadQuery> for ReadManyRecordsBuilder {
    fn build(self) -> QueryGraphBuilderResult<ReadQuery> {
        let args = self.args;
        let name = self.field.name;
        let alias = self.field.alias;

        let nested_fields = self.field.nested_fields.unwrap().fields;
        let selection_order: Vec<String> = collect_selection_order(&nested_fields);
        let selected_fields = collect_selected_fields(&nested_fields, &self.model, None);
        let nested = collect_nested_queries(nested_fields, &self.model)?;
        let model = self.model;

        Ok(ReadQuery::ManyRecordsQuery(ManyRecordsQuery {
            name,
            alias,
            model,
            args,
            selected_fields,
            nested,
            selection_order,
        }))
    }
}
