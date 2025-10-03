use query_engine_tests::*;

// This test inserts lots of m2m relations to specifically test the chunking logic for
// dynamically generated rows in the query compiler.
// Planetscale and MySQL via MariaDB driver consistently time out when running this test.
#[test_suite(
    schema(schema),
    exclude(
        MongoDb,
        Vitess("planetscale.js.wasm"),
        Mysql("mariadb.js.wasm"),
        Mysql("mariadb-mysql.js.wasm")
    )
)]
mod chunking_qc {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
          r#"
            model User {
                #id(id, Int, @id)
                roles Role[]
            }

            model Role {
                #id(id, Int, @id)
                users User[]
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn create_lots_of_m2m_relations(runner: Runner) -> TestResult<()> {
        // Each relation requires 2 bind values (one for each side of the relation),
        // so we divide the limit by 2 and add 1 to ensure we exceed the limit.
        let relation_count: usize = runner.max_bind_values().unwrap_or(65535) / 2 + 1;

        let result = runner
            .query(format!(
                r#"
              mutation {{
                createOneUser(data: {{
                  id: 1,
                  roles: {{
                    create: [{}]
                  }}
                }}) {{
                  roles {{
                    id
                  }}
                }}
              }}
              "#,
                (1..=relation_count)
                    .map(|i| format!("{{ id: {i} }}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .await?;

        result.assert_success();

        let rows = result.into_data();
        let roles = rows[0]
            .as_object()
            .unwrap()
            .get("createOneUser")
            .unwrap()
            .as_object()
            .unwrap()
            .get("roles")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(roles.len(), relation_count);
        Ok(())
    }
}
