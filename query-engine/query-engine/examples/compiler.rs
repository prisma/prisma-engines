use std::sync::Arc;

use indexmap::IndexMap;
use query_core::{query_graph_builder::QueryGraphBuilder, schema::QueryTag, QueryDocument};
use request_handlers::{Action, FieldQuery, JsonBody, JsonSingleQuery, RequestBody, SelectionSet, SelectionSetValue};

pub fn main() -> anyhow::Result<()> {
    let schema_path = std::env::var("PRISMA_DML_PATH")?;
    let schema_string = std::fs::read_to_string(schema_path)?;
    let schema = psl::validate(schema_string.into());

    if schema.diagnostics.has_errors() {
        anyhow::bail!("invalid schema");
    }

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let request = RequestBody::Json(JsonBody::Single(JsonSingleQuery {
        model_name: Some("User".into()),
        action: Action::new(QueryTag::FindMany),
        query: FieldQuery {
            arguments: None,
            selection: SelectionSet::new({
                let mut map = IndexMap::new();
                map.insert("$scalars".into(), SelectionSetValue::Shorthand(true));
                map
            }),
        },
    }));

    let doc = request.into_doc(&query_schema)?;

    let QueryDocument::Single(query) = doc else {
        anyhow::bail!("expected single query");
    };

    let (graph, _serializer) = QueryGraphBuilder::new(&query_schema).build(query)?;

    println!("{}", graph.to_string());

    let expr = query_core::compiler::translate(graph);

    println!("{expr:?}");

    Ok(())
}
