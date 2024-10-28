use std::sync::Arc;

use indexmap::IndexMap;
use query_core::{query_graph_builder::QueryGraphBuilder, schema::QueryTag, QueryDocument};
use request_handlers::{Action, FieldQuery, JsonBody, JsonSingleQuery, RequestBody, SelectionSet, SelectionSetValue};

pub fn main() -> anyhow::Result<()> {
    let schema_string = include_str!("./schema.prisma");
    let schema = psl::validate(schema_string.into());

    if schema.diagnostics.has_errors() {
        anyhow::bail!("invalid schema");
    }

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    // prisma.user.findMany({
    //     where: {
    //         email: Prisma.Param("userEmail")
    //     }
    // })
    let request = RequestBody::Json(JsonBody::Single(JsonSingleQuery {
        model_name: Some("User".into()),
        action: Action::new(QueryTag::FindMany),
        query: FieldQuery {
            arguments: Some({
                let mut map = IndexMap::new();
                map.insert(
                    "where".into(),
                    serde_json::json!({
                        "email": {
                            "$type": "Param",
                            "value": "userEmail",
                        }
                    }),
                );
                map
            }),
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

    println!("{graph}");

    let expr = query_core::compiler::translate(graph)?;

    println!("{expr}");

    Ok(())
}