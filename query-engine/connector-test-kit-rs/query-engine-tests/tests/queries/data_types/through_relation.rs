use indoc::indoc;
use query_engine_tests::*;

#[test_suite]
mod scalar_relations {
    fn schema_common() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)

              children Child[]
            }
            
            model Child {
              #id(childId, Int, @id)

              parentId Int?
              parent Parent? @relation(fields: [parentId], references: [id])

              string  String
              int     Int
              bInt    BigInt
              float   Float
              bytes   Bytes
              bool    Boolean
              dt      DateTime
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_common))]
    async fn common_types(runner: Runner) -> TestResult<()> {
        create_common_children(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id children { childId string int bInt float bytes bool dt } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"string":"abc","int":1,"bInt":"1","float":1.5,"bytes":"AQID","bool":false,"dt":"1900-10-10T01:10:10.001Z"},{"childId":2,"string":"def","int":-4234234,"bInt":"14324324234324","float":-2.54367,"bytes":"FDSF","bool":true,"dt":"1999-12-12T21:12:12.121Z"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueParent(where: { id: 1 }) { id children { childId string int bInt float bytes bool dt } } }"#),
          @r###"{"data":{"findUniqueParent":{"id":1,"children":[{"childId":1,"string":"abc","int":1,"bInt":"1","float":1.5,"bytes":"AQID","bool":false,"dt":"1900-10-10T01:10:10.001Z"},{"childId":2,"string":"def","int":-4234234,"bInt":"14324324234324","float":-2.54367,"bytes":"FDSF","bool":true,"dt":"1999-12-12T21:12:12.121Z"}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueParent(where: { id: 2 }) { id children { childId string int bInt float bytes bool dt } } }"#),
          @r###"{"data":{"findUniqueParent":null}}"###
        );

        Ok(())
    }

    fn schema_json() -> String {
        let schema = indoc! {
            r#"model Parent {
            #id(id, Int, @id)

            children Child[]
          }
          
          model Child {
            #id(childId, Int, @id)

            parentId Int?
            parent Parent? @relation(fields: [parentId], references: [id])

            json Json
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_json), capabilities(Json), exclude(Mysql(5.6)))]
    async fn json_type(runner: Runner) -> TestResult<()> {
        create_child(&runner, r#"{ childId: 1, json: "1" }"#).await?;
        create_child(&runner, r#"{ childId: 2, json: "{}" }"#).await?;
        create_child(&runner, r#"{ childId: 3, json: "{\"a\": \"b\"}" }"#).await?;
        create_child(&runner, r#"{ childId: 4, json: "[]" }"#).await?;
        create_child(&runner, r#"{ childId: 5, json: "[1, -1, true, {\"a\": \"b\"}]" }"#).await?;
        create_parent(
            &runner,
            r#"{ id: 1, children: { connect: [{ childId: 1 }, { childId: 2 }, { childId: 3 }, { childId: 4 }, { childId: 5 }] } }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(orderBy: { id: asc }) { id children { childId json } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"json":"1"},{"childId":2,"json":"{}"},{"childId":3,"json":"{\"a\":\"b\"}"},{"childId":4,"json":"[]"},{"childId":5,"json":"[1,-1,true,{\"a\":\"b\"}]"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueParent(where: { id: 1 }) { id children { childId json } } }"#),
          @r###"{"data":{"findUniqueParent":{"id":1,"children":[{"childId":1,"json":"1"},{"childId":2,"json":"{}"},{"childId":3,"json":"{\"a\":\"b\"}"},{"childId":4,"json":"[]"},{"childId":5,"json":"[1,-1,true,{\"a\":\"b\"}]"}]}}}"###
        );

        Ok(())
    }

    fn schema_enum() -> String {
        let schema = indoc! {
            r#"model Parent {
                #id(id, Int, @id)

                children Child[]
              }
              
              model Child {
                #id(childId, Int, @id)

                parentId Int?
                parent Parent? @relation(fields: [parentId], references: [id])

                enum Color
              }

              enum Color {
                Red
                Green
                Blue
              }
        "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_enum), capabilities(Enums))]
    async fn enum_type(runner: Runner) -> TestResult<()> {
        create_child(&runner, r#"{ childId: 1, enum: Red }"#).await?;
        create_child(&runner, r#"{ childId: 2, enum: Green }"#).await?;
        create_child(&runner, r#"{ childId: 3, enum: Blue }"#).await?;
        create_parent(
            &runner,
            r#"{ id: 1, children: { connect: [{ childId: 1 }, { childId: 2 }, { childId: 3 }] } }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(orderBy: { id :asc }) { id children { childId enum } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"enum":"Red"},{"childId":2,"enum":"Green"},{"childId":3,"enum":"Blue"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueParent(where: { id: 1 }) { id children { childId enum } } }"#),
          @r###"{"data":{"findUniqueParent":{"id":1,"children":[{"childId":1,"enum":"Red"},{"childId":2,"enum":"Green"},{"childId":3,"enum":"Blue"}]}}}"###
        );

        Ok(())
    }

    fn schema_decimal() -> String {
        let schema = indoc! {
            r#"model Parent {
              #id(id, Int, @id)

              children Child[]
            }
            
            model Child {
              #id(childId, Int, @id)

              parentId Int?
              parent Parent? @relation(fields: [parentId], references: [id])

              dec Decimal
            }
      "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_decimal), capabilities(DecimalType), exclude(Sqlite("cfd1")))]
    // On D1, this fails with:
    //
    // ```diff
    // - {"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"dec":"1"},{"childId":2,"dec":"-1"},{"childId":3,"dec":"123.4567891"},{"childId":4,"dec":"95993.57"}]}]}}
    // + {"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"dec":"1"},{"childId":2,"dec":"-1"},{"childId":3,"dec":"123.4567891"},{"childId":4,"dec":"95993.57000000001"}]}]}}
    // ```
    //
    // Basically, decimals are treated as doubles (and lose precision) due to D1 not providing column type information on queries.
    async fn decimal_type(runner: Runner) -> TestResult<()> {
        create_child(&runner, r#"{ childId: 1, dec: "1" }"#).await?;
        create_child(&runner, r#"{ childId: 2, dec: "-1" }"#).await?;
        create_child(&runner, r#"{ childId: 3, dec: "123.45678910" }"#).await?;
        create_child(&runner, r#"{ childId: 4, dec: "95993.57" }"#).await?;
        create_parent(
            &runner,
            r#"{ id: 1, children: { connect: [{ childId: 1 }, { childId: 2 }, { childId: 3 }, { childId: 4 }] } }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent(orderBy: { id: asc }) { id children { childId dec } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"dec":"1"},{"childId":2,"dec":"-1"},{"childId":3,"dec":"123.4567891"},{"childId":4,"dec":"95993.57"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueParent(where: { id: 1 }) { id children { childId dec } } }"#),
          @r###"{"data":{"findUniqueParent":{"id":1,"children":[{"childId":1,"dec":"1"},{"childId":2,"dec":"-1"},{"childId":3,"dec":"123.4567891"},{"childId":4,"dec":"95993.57"}]}}}"###
        );

        Ok(())
    }

    fn schema_scalar_lists() -> String {
        let schema = indoc! {
            r#"model Parent {
            #id(id, Int, @id)

            children Child[]
          }
          
          model Child {
            #id(childId, Int, @id)

            parentId Int?
            parent Parent? @relation(fields: [parentId], references: [id])

            string  String[]
            int     Int[]
            bInt    BigInt[]
            float   Float[]
            bytes   Bytes[]
            bool    Boolean[]
            dt      DateTime[]
            empty   Int[]
            unset   Int[]
          }
          "#
        };

        schema.to_owned()
    }

    // TODO: fix https://github.com/prisma/team-orm/issues/684 and unexclude DAs
    // On "pg.js.wasm", this fails with a `QueryParserError` due to bigint issues.
    #[connector_test(
        schema(schema_scalar_lists),
        capabilities(ScalarLists),
        exclude(Postgres("pg.js", "neon.js", "pg.js.wasm", "neon.js.wasm"))
    )]
    async fn scalar_lists(runner: Runner) -> TestResult<()> {
        create_child(
            &runner,
            r#"{
              childId: 1,
              string: ["abc", "def"],
              int: [1, -1, 1234567],
              bInt: [1, -1, 9223372036854775807, -9223372036854775807],
              float: [1.5, -1.5, 1.234567],
              bytes: ["AQID", "Qk9OSk9VUg=="],
              bool: [false, true],
              dt: ["1900-10-10T01:10:10.001Z", "1999-12-12T21:12:12.121Z"],
              empty: []
          }"#,
        )
        .await?;
        create_parent(&runner, r#"{ id: 1, children: { connect: [{ childId: 1 }] } }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id children { childId string int bInt float bytes bool dt empty unset } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"string":["abc","def"],"int":[1,-1,1234567],"bInt":["1","-1","9223372036854775807","-9223372036854775807"],"float":[1.5,-1.5,1.234567],"bytes":["AQID","Qk9OSk9VUg=="],"bool":[false,true],"dt":["1900-10-10T01:10:10.001Z","1999-12-12T21:12:12.121Z"],"empty":[],"unset":[]}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueParent(where: { id: 1 }) { id children { childId string int bInt float bytes bool dt empty unset } } }"#),
          @r###"{"data":{"findUniqueParent":{"id":1,"children":[{"childId":1,"string":["abc","def"],"int":[1,-1,1234567],"bInt":["1","-1","9223372036854775807","-9223372036854775807"],"float":[1.5,-1.5,1.234567],"bytes":["AQID","Qk9OSk9VUg=="],"bool":[false,true],"dt":["1900-10-10T01:10:10.001Z","1999-12-12T21:12:12.121Z"],"empty":[],"unset":[]}]}}}"###
        );

        Ok(())
    }

    fn schema_oid() -> String {
        let schema = indoc! {
            r#"model Parent {
            #id(id, Int, @id)

            children Child[]
          }
          
          model Child {
            #id(childId, Int, @id)

            parentId Int?
            parent Parent? @relation(fields: [parentId], references: [id])

            oid Int @test.Oid
          }
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_oid), only(Postgres, CockroachDb))]
    async fn oid_type(runner: Runner) -> TestResult<()> {
        create_child(&runner, r#"{ childId: 1, oid: 0 }"#).await?;
        create_child(&runner, r#"{ childId: 2, oid: 1 }"#).await?;
        create_child(&runner, r#"{ childId: 3, oid: 65587 }"#).await?;
        create_child(&runner, &format!(r#"{{ childId: 4, oid: {} }}"#, u32::MAX)).await?;
        create_parent(
            &runner,
            r#"{ id: 1, children: { connect: [{ childId: 1 }, { childId: 2 }, { childId: 3 }, { childId: 4 }] } }"#,
        )
        .await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyParent { id children(orderBy: { oid: asc }) { oid } } }"#),
          @r###"{"data":{"findManyParent":[{"id":1,"children":[{"oid":0},{"oid":1},{"oid":65587},{"oid":4294967295}]}]}}"###
        );

        Ok(())
    }

    async fn create_common_children(runner: &Runner) -> TestResult<()> {
        create_child(
            runner,
            r#"{
          childId: 1,
          string: "abc",
          int: 1,
          bInt: 1,
          float: 1.5,
          bytes: "AQID",
          bool: false,
          dt: "1900-10-10T01:10:10.001Z",
      }"#,
        )
        .await?;

        create_child(
            runner,
            r#"{
          childId: 2,
          string: "def",
          int: -4234234,
          bInt: 14324324234324,
          float: -2.54367,
          bytes: "FDSF",
          bool: true,
          dt: "1999-12-12T21:12:12.121Z",
        }"#,
        )
        .await?;

        create_parent(
            runner,
            r#"{ id: 1, children: { connect: [{ childId: 1 }, { childId: 2 }] } }"#,
        )
        .await?;

        Ok(())
    }

    async fn create_child(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneChild(data: {}) {{ childId }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }

    async fn create_parent(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
