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

#[test]
fn kingbase_mysql_bit_parameters_keep_declared_width_without_changing_mysql() {
    fn collect_types(value: &serde_json::Value, types: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(fields) => {
                if let Some(serde_json::Value::String(db_type)) = fields.get("dbType") {
                    types.push(db_type.clone());
                }
                for value in fields.values() {
                    collect_types(value, types);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect_types(value, types);
                }
            }
            _ => (),
        }
    }

    for kingbase in [false, true] {
        let provider = if kingbase { "kingbase-mysql" } else { "mysql" };
        let schema = psl::validate_without_extensions(
            format!(
                "datasource db {{\n provider = \"{provider}\"\n}}\n\
                 model Item {{\n id Int @id\n small Bytes @db.Bit(9)\n wide Bytes @db.Bit(64)\n}}"
            )
            .into(),
        );
        assert!(!schema.diagnostics.has_errors(), "{:?}", schema.diagnostics);
        let query_schema = Arc::new(query_core::schema::build(Arc::new(schema), true));
        let info = ExternalConnectionInfo::new(SqlFamily::Mysql, None, None, false);
        let info = ConnectionInfo::External(if kingbase { info.with_kingbase_mysql() } else { info });
        let small = serde_json::json!({ "$type": "Bytes", "value": "AAE=" });
        let wide = serde_json::json!({ "$type": "Bytes", "value": "AAAAAAAGazo=" });
        let data = serde_json::json!({ "id": 1, "small": small, "wide": wide });
        for (action, arguments) in [
            ("createOne", serde_json::json!({ "data": data })),
            ("createMany", serde_json::json!({ "data": [data.clone(), data] })),
            (
                "updateOne",
                serde_json::json!({ "where": { "id": 1 }, "data": { "small": small, "wide": wide } }),
            ),
            (
                "findMany",
                serde_json::json!({ "where": { "small": { "in": [small] }, "wide": wide } }),
            ),
        ] {
            let request = serde_json::json!({
                "modelName": "Item", "action": action,
                "query": { "arguments": arguments, "selection": if action == "createMany" {
                    serde_json::json!({ "count": true })
                } else { serde_json::json!({ "id": true }) } }
            });
            let request =
                RequestBody::try_from_str(&request.to_string(), query_core::protocol::EngineProtocol::Json).unwrap();
            let QueryDocument::Single(operation) = request.into_doc(&query_schema).unwrap() else {
                panic!()
            };
            let plan = serde_json::to_value(query_compiler::compile(&query_schema, operation, &info).unwrap()).unwrap();
            let mut types = Vec::new();
            collect_types(&plan, &mut types);
            if kingbase {
                assert!(types.iter().any(|typ| typ == "BIT(9)"), "{action}: {plan}");
                assert!(types.iter().any(|typ| typ == "BIT(64)"), "{action}: {plan}");
            } else {
                assert!(types.iter().any(|typ| typ == "BIT"), "{action}: {plan}");
                assert!(!types.iter().any(|typ| typ.starts_with("BIT(")), "{action}: {plan}");
            }
        }
    }
}
