#![cfg(feature = "kingbase-mysql")]

use quaint::prelude::{ConnectionInfo, ExternalConnectionInfo, SqlFamily};
use query_core::QueryDocument;
use query_structure::psl;
use request_handlers::RequestBody;
use std::sync::Arc;

#[test]
fn kingbase_mysql_full_text_search_uses_native_tsquery_without_parameter_markers() {
    let schema = psl::validate_without_extensions(
        r#"
        datasource db {
          provider = "kingbase-mysql"
        }
        model Item {
          id Int @id
          name String
          @@fulltext([name])
        }
        "#
        .into(),
    );
    assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);
    let query_schema = Arc::new(query_core::schema::build(Arc::new(schema), true));
    let info = ConnectionInfo::External(
        ExternalConnectionInfo::new(SqlFamily::Mysql, None, None, false).with_kingbase_mysql(),
    );

    for arguments in [
        serde_json::json!({ "where": { "name": { "search": "John & Smith" } } }),
        serde_json::json!({ "orderBy": { "_relevance": { "fields": ["name"], "search": "John | Smith", "sort": "desc" } } }),
    ] {
        let request = serde_json::json!({
            "modelName": "Item", "action": "findMany",
            "query": { "arguments": arguments, "selection": { "id": true, "name": true } }
        });
        let request =
            RequestBody::try_from_str(&request.to_string(), query_core::protocol::EngineProtocol::Json).unwrap();
        let QueryDocument::Single(operation) = request.into_doc(&query_schema).unwrap() else {
            panic!()
        };
        let rendered =
            serde_json::to_string(&query_compiler::compile(&query_schema, operation, &info).unwrap()).unwrap();
        assert!(rendered.contains("to_tsquery"), "{rendered}");
        assert!(!rendered.contains("PRISMA_MYSQL_BOOLEAN_QUERY"), "{rendered}");
        assert!(!rendered.contains("MATCH("), "{rendered}");
    }
}

#[test]
fn kingbase_mysql_explicit_autoincrement_ids_have_adapter_metadata() {
    for kingbase in [false, true] {
        let provider = if kingbase { "kingbase-mysql" } else { "mysql" };
        let schema = psl::validate_without_extensions(
            format!(
                "datasource db {{\n provider = \"{provider}\"\n}}\n\
                 model Item {{\n id Int @id @default(autoincrement()) @map(\"item_key\")\n}}"
            )
            .into(),
        );
        assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);
        let query_schema = Arc::new(query_core::schema::build(Arc::new(schema), true));
        let info = ExternalConnectionInfo::new(SqlFamily::Mysql, None, None, false);
        let info = ConnectionInfo::External(if kingbase { info.with_kingbase_mysql() } else { info });

        for explicit in [false, true] {
            let data = if explicit {
                serde_json::json!({ "id": 100 })
            } else {
                serde_json::json!({})
            };
            let request = serde_json::json!({
                "modelName": "Item", "action": "createOne",
                "query": { "arguments": { "data": data }, "selection": { "id": true } }
            });
            let request =
                RequestBody::try_from_str(&request.to_string(), query_core::protocol::EngineProtocol::Json).unwrap();
            let QueryDocument::Single(operation) = request.into_doc(&query_schema).unwrap() else {
                panic!()
            };
            let plan = query_compiler::compile(&query_schema, operation, &info).unwrap();
            let rendered = serde_json::to_string(&plan).unwrap();
            assert_eq!(
                rendered.contains("PRISMA_MYSQL_AUTOINCREMENT_ID"),
                kingbase && explicit,
                "{rendered}"
            );
        }
    }
}
