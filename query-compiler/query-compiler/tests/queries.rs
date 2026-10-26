use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Postgres,
};
use query_core::ToGraphviz;
use request_handlers::JsonSingleQuery;
use sql_query_builder::{Context, SqlQueryBuilder};
use std::{fs, process::Command};

mod common;

#[test]
fn queries() {
    insta::glob!("data/*.json", |path| {
        let test_name = path.file_name().unwrap().display();
        println!("running: {test_name}");

        let schema_string = include_str!("data/schema.prisma");
        let connection_info = ConnectionInfo::External(ExternalConnectionInfo::new(
            SqlFamily::Postgres,
            Some("public".to_owned()),
            None,
            true,
        ));

        let query = fs::read_to_string(path).unwrap();
        let query: JsonSingleQuery = serde_json::from_str(&query).unwrap();

        let graph = common::query_graph(schema_string, query);

        let dot = graph.to_graphviz();
        let tests_path = path.parent().unwrap().parent().unwrap();
        let graphs_path = tests_path.join("graphs");
        let dot_path = graphs_path.join(path.file_name().unwrap()).with_extension("dot");

        std::thread::spawn(move || {
            fs::create_dir_all(graphs_path).unwrap();
            fs::write(&dot_path, dot).unwrap();

            if std::env::var("RENDER_DOT_TO_PNG").is_ok() {
                let png_path = dot_path.with_extension("png");
                Command::new("dot")
                    .arg("-Tpng")
                    .arg(dot_path)
                    .arg("-Gdpi=300")
                    .arg("-Nfontname=Helvetica")
                    .arg("-Efontname=Helvetica")
                    .arg("-o")
                    .arg(png_path)
                    .status()
                    .unwrap();
            }
        });

        let ctx = Context::new(&connection_info, None);
        let builder = SqlQueryBuilder::<Postgres<'_>>::new(ctx);

        let expr = query_compiler::translate(graph, &builder).unwrap();
        let pretty = expr.pretty_print(false, 80).unwrap();
        insta::assert_snapshot!(pretty);
    });
}
