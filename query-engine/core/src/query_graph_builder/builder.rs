use once_cell::sync::Lazy;
use std::{fmt, fs::File, io::Write};

use super::*;
use crate::{query_document::*, query_graph::*, schema::*, IrSerializer};

pub static PRISMA_RENDER_DOT_FILE: Lazy<bool> = Lazy::new(|| match std::env::var("PRISMA_RENDER_DOT_FILE") {
    Ok(enabled) => enabled == *("true") || enabled == *("1"),
    Err(_) => false,
});

pub struct QueryGraphBuilder<'a> {
    query_schema: &'a QuerySchema,
}

impl fmt::Debug for QueryGraphBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryGraphBuilder").finish()
    }
}

impl<'a> QueryGraphBuilder<'a> {
    pub fn new(query_schema: &'a QuerySchema) -> Self {
        Self { query_schema }
    }

    /// Maps an operation to a query.
    pub fn build(self, operation: Operation) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer<'a>)> {
        let _span = info_span!("prisma:engine:build_graph");
        match operation {
            Operation::Read(selection) => self.build_internal(selection, self.query_schema.query(), &|name| {
                self.query_schema.find_query_field(name)
            }),
            Operation::Write(selection) => self.build_internal(selection, self.query_schema.mutation(), &|name| {
                self.query_schema.find_mutation_field(name)
            }),
        }
    }

    fn build_internal(
        &self,
        selection: Selection,
        root_object: ObjectType<'a>, // Either the query or mutation object.
        root_object_fields: &dyn Fn(&str) -> Option<OutputField<'a>>,
    ) -> QueryGraphBuilderResult<(QueryGraph, IrSerializer<'a>)> {
        let mut selections = vec![selection];
        let mut parsed_object = QueryDocumentParser::new(crate::executor::get_request_now()).parse(
            &selections,
            &root_object,
            root_object_fields,
            self.query_schema,
        )?;

        // Because we're processing root objects, there can only be one query / mutation.
        let field_pair = parsed_object.fields.pop().unwrap();
        let serializer = Self::derive_serializer(&selections.pop().unwrap(), field_pair.schema_field.clone());

        if field_pair.schema_field.query_info().is_some() {
            let graph = self.dispatch_build(field_pair)?;
            Ok((graph, serializer))
        } else {
            Err(QueryGraphBuilderError::SchemaError(format!(
                "Expected query information to be attached on schema object '{}', field '{}'.",
                root_object.name(),
                field_pair.parsed_field.name
            )))
        }
    }

    #[rustfmt::skip]
    fn dispatch_build(&self, field_pair: FieldPair<'a>) -> QueryGraphBuilderResult<QueryGraph> {
        let query_info = field_pair.schema_field.query_info().unwrap();
        let parsed_field = field_pair.parsed_field;
        let query_schema = self.query_schema;

        let mut graph = match (&query_info.tag, query_info.model.map(|id| self.query_schema.internal_data_model.clone().zip(id))) {
            (QueryTag::FindUnique, Some(m)) => read::find_unique(parsed_field, m).map(Into::into),
            (QueryTag::FindUniqueOrThrow, Some(m)) => read::find_unique_or_throw(parsed_field, m).map(Into::into),
            (QueryTag::FindFirst, Some(m)) => read::find_first(parsed_field, m).map(Into::into),
            (QueryTag::FindFirstOrThrow, Some(m)) => read::find_first_or_throw(parsed_field, m).map(Into::into),
            (QueryTag::FindMany, Some(m)) => read::find_many(parsed_field, m).map(Into::into),
            (QueryTag::Aggregate, Some(m)) => read::aggregate(parsed_field, m).map(Into::into),
            (QueryTag::GroupBy, Some(m)) => read::group_by(parsed_field, m).map(Into::into),
            (QueryTag::CreateOne, Some(m)) => QueryGraph::root(|g| write::create_record(g, query_schema, m, parsed_field)),
            (QueryTag::CreateMany, Some(m)) => QueryGraph::root(|g| write::create_many_records(g, query_schema,m, parsed_field)),
            (QueryTag::UpdateOne, Some(m)) => QueryGraph::root(|g| write::update_record(g, query_schema, m, parsed_field)),
            (QueryTag::UpdateMany, Some(m)) => QueryGraph::root(|g| write::update_many_records(g, query_schema, m, parsed_field)),
            (QueryTag::UpsertOne, Some(m)) => QueryGraph::root(|g| write::upsert_record(g, query_schema, m, parsed_field)),
            (QueryTag::DeleteOne, Some(m)) => QueryGraph::root(|g| write::delete_record(g, query_schema, m, parsed_field)),
            (QueryTag::DeleteMany, Some(m)) => QueryGraph::root(|g| write::delete_many_records(g, query_schema, m, parsed_field)),
            (QueryTag::ExecuteRaw, _) => QueryGraph::root(|g| write::execute_raw(g, parsed_field)),
            (QueryTag::QueryRaw, m) => QueryGraph::root(|g| write::query_raw(g, m, None, parsed_field)),
            // MongoDB specific raw operations
            (QueryTag::FindRaw, m) => QueryGraph::root(|g| write::query_raw(g, m, Some(QueryTag::FindRaw.to_string()), parsed_field)),
            (QueryTag::AggregateRaw, m) => QueryGraph::root(|g| write::query_raw(g, m, Some(QueryTag::AggregateRaw.to_string()), parsed_field)),
            (QueryTag::RunCommandRaw, m) => QueryGraph::root(|g| write::query_raw(g, m, Some(QueryTag::RunCommandRaw.to_string()), parsed_field)),
            _ => unreachable!("Query builder dispatching failed."),
        }?;

        // Run final transformations.
        graph.finalize(self.query_schema.capabilities())?;
        trace!("{}", graph);

        // Used to debug generated graph.
        if *PRISMA_RENDER_DOT_FILE {
            let mut f = File::create("graph.dot").unwrap();
            let output = graph.to_graphviz();

            f.write_all(output.as_bytes()).unwrap();
        }

        Ok(graph)
    }

    fn derive_serializer(selection: &Selection, field: OutputField<'a>) -> IrSerializer<'a> {
        IrSerializer {
            key: selection
                .alias()
                .clone()
                .unwrap_or_else(|| selection.name().to_string()),
            output_field: field,
        }
    }
}
