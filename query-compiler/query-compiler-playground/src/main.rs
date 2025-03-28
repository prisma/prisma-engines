use std::{fs, path::PathBuf, process::Command, sync::Arc};

use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Postgres,
};
use query_core::{QueryDocument, QueryGraph, ToGraphviz, query_graph_builder::QueryGraphBuilder};
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};
use serde_json::json;
use sql_query_builder::{Context, SqlQueryBuilder};

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

    let (graph, _serializer) = QueryGraphBuilder::new(&query_schema)
        .without_eager_default_evaluation()
        .build(query)?;

    println!("{graph}");
    render_query_graph(&graph)?;

    let ctx = Context::new(&connection_info, None);
    let builder = SqlQueryBuilder::<Postgres<'_>>::new(ctx);

    let expr = query_compiler::translate(graph, &builder)?;

    println!("{}", expr.pretty_print(true, 80)?);

    Ok(())
}

fn render_query_graph(graph: &QueryGraph) -> anyhow::Result<()> {
    let package_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dot_path = package_root.join("graph.dot");
    let png_path = package_root.join("graph.png");

    fs::write(&dot_path, graph.to_graphviz())?;

    match Command::new("dot")
        .arg("-Tpng")
        .arg(&dot_path)
        .arg("-o")
        .arg(&png_path)
        .status()
    {
        Ok(status) if !status.success() => {
            anyhow::bail!("graphviz exited with status {status}")
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("`dot` command not found, please install graphviz to render query graphs");
            Ok(())
        }
        Err(err) => {
            anyhow::bail!("failed to execute graphviz: {err}")
        }
    }
}
