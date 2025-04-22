use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Postgres,
};
use query_core::{QueryDocument, QueryGraphBuilder, ToGraphviz};
use query_structure::psl;
use request_handlers::{JsonBody, JsonSingleQuery, RequestBody};
use sql_query_builder::{Context, SqlQueryBuilder};
use std::path::PathBuf;
use std::{fs, sync::Arc};

#[test]
fn queries() {
    insta::glob!("data/*.json", |path| {
        let schema_string = include_str!("data/schema.prisma");
        let schema = psl::validate(schema_string.into());

        assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);

        let schema = Arc::new(schema);
        let query_schema = Arc::new(query_core::schema::build(schema, true));

        let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
            SqlFamily::Postgres,
            "public".to_owned(),
            None,
        ));

        let query = fs::read_to_string(path).unwrap();
        let query: JsonSingleQuery = serde_json::from_str(&query).unwrap();

        let request = RequestBody::Json(JsonBody::Single(query));
        let doc = request.into_doc(&query_schema).unwrap();

        let QueryDocument::Single(query) = doc else {
            panic!("expected single query");
        };

        let (graph, _serializer) = QueryGraphBuilder::new(&query_schema)
            .without_eager_default_evaluation()
            .build(query)
            .unwrap();

        let dot = graph.to_graphviz();
        let tests_path = path.parent().unwrap().parent().unwrap();
        let graphs_path = tests_path.join("graphs");
        let dot_path = graphs_path.join(path.file_name().unwrap()).with_extension("dot");
        fs::create_dir_all(graphs_path).unwrap();
        fs::write(dot_path, dot).unwrap();

        let ctx = Context::new(&connection_info, None);
        let builder = SqlQueryBuilder::<Postgres<'_>>::new(ctx);

        let expr = query_compiler::translate(graph, &builder).unwrap();
        let pretty = expr.pretty_print(false, 80).unwrap();
        insta::assert_snapshot!(pretty);
    });
}
