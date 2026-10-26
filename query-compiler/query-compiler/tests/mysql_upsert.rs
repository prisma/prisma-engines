use quaint::{
    prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily},
    visitor::Mysql,
};
use request_handlers::JsonSingleQuery;
use serde_json::{Value, json};
use sql_query_builder::{Context, SqlQueryBuilder};

mod common;

const COUNTER: &str = r#"
model Counter {
  id     String @id @map("counter_id")
  clicks Int

  @@map("counters")
}
"#;

fn compile_upsert(model: &str, relation_mode: &str, arguments: Value) -> String {
    let schema = format!("datasource db {{\n provider = \"mysql\"\n relationMode = \"{relation_mode}\"\n}}\n{model}");
    let query: JsonSingleQuery = serde_json::from_value(json!({
        "modelName": "Counter",
        "action": "upsertOne",
        "query": {
            "arguments": arguments,
            "selection": { "id": true, "clicks": true }
        }
    }))
    .unwrap();
    let graph = common::query_graph(&schema, query);
    let connection = ConnectionInfo::External(ExternalConnectionInfo::new(SqlFamily::Mysql, None, None, false));
    let builder = SqlQueryBuilder::<Mysql<'_>>::new(Context::new(&connection, None));
    query_compiler::translate(graph, &builder)
        .unwrap()
        .pretty_print(false, 120)
        .unwrap()
}

fn arguments() -> Value {
    json!({
        "where": { "id": "counter" },
        "create": { "id": "counter", "clicks": 1 },
        "update": { "clicks": { "increment": 1 } }
    })
}

#[test]
fn atomic_write_precedes_read_back_in_both_relation_modes() {
    for relation_mode in ["foreignKeys", "prisma"] {
        let plan = compile_upsert(COUNTER, relation_mode, arguments());
        insta::assert_snapshot!(format!("atomic_upsert_{relation_mode}"), plan);
    }
}

#[test]
fn a_compound_primary_key_can_be_read_back() {
    let model = r#"
model Counter {
  scope  String
  id     String @map("counter_id")
  clicks Int

  @@id([scope, id])
  @@map("counters")
}
"#;
    let plan = compile_upsert(
        model,
        "prisma",
        json!({
            "where": { "scope_id": { "scope": "scope", "id": "counter" } },
            "create": { "scope": "scope", "id": "counter", "clicks": 1 },
            "update": { "clicks": { "increment": 1 } }
        }),
    );
    insta::assert_snapshot!(plan);
}

#[test]
fn an_automatically_updated_lookup_key_keeps_the_emulated_path() {
    let model = "model Counter {\n id DateTime @id @updatedAt\n clicks Int\n}";
    let id = "2020-01-01T00:00:00Z";
    let plan = compile_upsert(
        model,
        "foreignKeys",
        json!({
            "where": { "id": id },
            "create": { "id": id, "clicks": 1 },
            "update": { "clicks": { "increment": 1 } }
        }),
    );
    insta::assert_snapshot!(plan);
}

#[test]
fn other_unique_constraints_keep_the_emulated_path() {
    let schema = COUNTER.replace("clicks Int", "clicks Int\n other String? @unique");
    let plan = compile_upsert(&schema, "foreignKeys", arguments());
    insta::assert_snapshot!(plan);
}

#[test]
fn prefix_keys_keep_the_emulated_path() {
    for (name, key) in [("primary_key", "@id(length: 5)"), ("unique", "@unique(length: 5)")] {
        let model = format!("model Counter {{\n id String {key} @db.VarChar(20)\n clicks Int\n}}");
        let plan = compile_upsert(&model, "foreignKeys", arguments());
        insta::assert_snapshot!(format!("prefix_{name}"), plan);
    }
}

#[test]
fn changing_the_lookup_key_keeps_the_emulated_path() {
    let mut arguments = arguments();
    arguments["update"]["id"] = json!("other-counter");
    let plan = compile_upsert(COUNTER, "foreignKeys", arguments);
    insta::assert_snapshot!(plan);
}

#[test]
fn additional_predicates_keep_the_emulated_path() {
    let mut arguments = arguments();
    arguments["where"]["clicks"] = json!(1);
    let plan = compile_upsert(COUNTER, "foreignKeys", arguments);
    insta::assert_snapshot!(plan);
}

#[test]
fn empty_update_keeps_the_emulated_path() {
    let mut arguments = arguments();
    arguments["update"] = json!({});
    let plan = compile_upsert(COUNTER, "foreignKeys", arguments);
    insta::assert_snapshot!(plan);
}

#[test]
fn models_with_relations_use_native_upsert_only_with_foreign_keys() {
    let model = format!(
        "{}\nmodel Child {{\n id Int @id\n counterId String\n counter Counter @relation(fields: [counterId], references: [id])\n}}",
        COUNTER.replace("clicks Int", "clicks Int\n children Child[]")
    );
    for relation_mode in ["foreignKeys", "prisma"] {
        let plan = compile_upsert(&model, relation_mode, arguments());
        insta::assert_snapshot!(format!("relations_{relation_mode}"), plan);
    }
}
