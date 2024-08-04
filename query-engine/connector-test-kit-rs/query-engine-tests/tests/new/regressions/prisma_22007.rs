use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod prisma_2207 {

    fn schema() -> String {
        r#"
            model Test {
                #id(id, Int, @id)
                title String
            }

        "#
        .to_owned()
    }
    #[connector_test]
    async fn filters_render_correctly(runner: Runner) -> TestResult<()> {
        let query = r#"
            mutation { 
                createManyTest( 
                    data: [
                        {id: 1, title: "a"}, 
                        {id: 2, title: "b"},
                        {id: 3, title: "c"}
                    ]
                ) {
                    count
                } 
            }"#;
        insta::assert_snapshot!(run_query!(runner, query), @r###"{"data":{"createManyTest":{"count":3}}}"###);

        let query = r#"
            query { 
                findManyTest( 
                    where: {
                        OR: [
                            { NOT: { title: "b" } },
                            { title: "c" }
                        ],
                    } 
                ) {
                    id
                    title
                } 
            }"#;

        insta::assert_snapshot!(run_query!(runner, query), @r###"{"data":{"findManyTest":[{"id":1,"title":"a"},{"id":3,"title":"c"}]}}"###);

        Ok(())
    }
}
