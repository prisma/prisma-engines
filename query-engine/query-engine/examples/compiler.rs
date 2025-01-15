use std::sync::Arc;

use quaint::connector::{ConnectionInfo, ExternalConnectionInfo, SqlFamily};
use query_core::{query_graph_builder::QueryGraphBuilder, QueryDocument};
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};
use serde_json::json;

pub fn main() -> anyhow::Result<()> {
    let schema_string = include_str!("./schema.prisma");
    let schema = psl::validate(schema_string.into());

    if schema.diagnostics.has_errors() {
        anyhow::bail!("invalid schema");
    }

    let schema = Arc::new(schema);
    let query_schema = Arc::new(query_core::schema::build(schema, true));

    let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
        SqlFamily::Postgres,
        "public".to_owned(),
        None,
    ));

    // prisma.user.findUnique({
    //     where: {
    //         email: Prisma.Param("userEmail")
    //     },
    //     select: {
    //         val: true,
    //         posts: true,
    //         profile: true,
    //     }
    // })
    let query: JsonSingleQuery = serde_json::from_value(json!({
        "modelName": "User",
        "action": "findMany",
        "query": {
            "arguments": {
                "where": {
                    "email": {
                        "$type": "Param",
                        "value": "userEmail"
                    }
                }
            },
            "selection": {
                "val": true,
                "posts": {
                    "arguments": {},
                    "selection": {
                        "$scalars": true
                    }
                },
                "profile": {
                    "arguments": {},
                    "selection": {
                        "$scalars": true
                    }
                }
            }
        }
    }))?;

    let request = RequestBody::Json(JsonBody::Single(query));
    let doc = request.into_doc(&query_schema)?;

    let QueryDocument::Single(query) = doc else {
        anyhow::bail!("expected single query");
    };

    let (graph, _serializer) = QueryGraphBuilder::new(&query_schema).build(query)?;

    println!("{graph}");

    let expr = query_core::compiler::translate(graph, &connection_info)?;

    println!("{}", expr.pretty_print(true, 80)?);

    Ok(())
}
