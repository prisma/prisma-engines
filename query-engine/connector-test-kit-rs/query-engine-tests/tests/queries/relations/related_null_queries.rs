use query_engine_tests::*;

#[test_suite]
mod inline_relation {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema_1() -> String {
        let schema = indoc! {
            r#"model ModelA {
              #id(id, String, @id)
              u  String?  @unique
              bs ModelB[]
            }
            
            model ModelB {
              #id(id, String, @id)
              a_u String?
              a   ModelA? @relation(fields: [a_u], references: [u])
            }"#
        };

        schema.to_owned()
    }

    fn schema_2() -> String {
        let schema = indoc! {
            r#"model ModelA {
            #id(id, String, @id)
            u1 String?
            u2 String?
            bs ModelB[]
          
            @@unique([u1, u2])
          }
          
          model ModelB {
            #id(id, String, @id)
            a_u1 String?
            a_u2 String?
            a    ModelA? @relation(fields: [a_u1, a_u2], references: [u1, u2])
          }"#
        };

        schema.to_owned()
    }

    fn schema_3() -> String {
        let schema = indoc! {
            r#"model ModelA {
            #id(id, String, @id)
            u  String? @unique
            b  ModelB?
          }
          
          model ModelB {
            #id(id, String, @id)
            a_u String?
            a   ModelA? @relation(fields: [a_u], references: [u])
          }"#
        };

        schema.to_owned()
    }

    fn schema_4() -> String {
        let schema = indoc! {
            r#"model ModelA {
          #id(id, String, @id)
          b_u String?
          b   ModelB? @relation(fields: [b_u], references: [u])
        }
        
        model ModelB {
          #id(id, String, @id)
          u  String? @unique
          a  ModelA?
        }"#
        };

        schema.to_owned()
    }

    // "Querying a single-field 1:n relation with nulls" should "ignore related records connected with null"
    #[connector_test(schema(schema_1))]
    async fn single_field_1n_rel_with_nulls(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelA(data: { id: "1", bs: { create: { id: "1" } } }){
              id
              bs {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"1","bs":[]}}}"###
        );

        Ok(())
    }

    // "Querying a multi-field 1:n relation with nulls" should "ignore related records connected with any null in the relation fields"
    #[connector_test(schema(schema_2))]
    async fn multi_field_1n_rel_with_nulls(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelA(data: { id: "1", bs: { create: { id: "1" } } }){
              id
              bs {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"1","bs":[]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelA(data: { id: "2", u1: "u1", bs: { create: { id: "2" } } }){
              id
              bs {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"2","bs":[]}}}"###
        );

        Ok(())
    }

    // "Querying a single-field 1:1 relation inlined on the child with null" should "not find a related record"
    #[connector_test(schema(schema_3))]
    async fn single_field_1_1_rel_inline_child(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelA(data: { id: "1", b: { create: { id: "1" } } }){
              id
              b {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"1","b":null}}}"###
        );

        Ok(())
    }

    // "Querying a single-field 1:1 relation inlined on the parent with null" should "not find a related record"
    #[connector_test(schema(schema_4))]
    async fn single_field_1_1_rel_inline_parent(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModelA(data: { id: "1", b: { create: { id: "1" } } }){
              id
              b {
                id
              }
            }
          }"#),
          @r###"{"data":{"createOneModelA":{"id":"1","b":null}}}"###
        );

        Ok(())
    }
}
