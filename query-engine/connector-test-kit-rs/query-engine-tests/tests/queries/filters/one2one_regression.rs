use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), exclude(SqlServer))]
mod one2one_regression {
    fn schema() -> String {
        let schema = indoc! {
            r#"
            model User {
                #id(id, Int, @id)
                name     String?
                friendOf User?   @relation("Userfriend")
                friend   User?   @relation("Userfriend", fields: [friendId], references: [id])
                friendId Int?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn work_with_nulls(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(runner, indoc! { r#"
                mutation {
                    createOneUser(data: { id: 1, name: "Bob"}) {
                        id
                        name
                        friend { name }
                        friendOf { name }
                    }
                }
            "#}),
            @r###"{"data":{"createOneUser":{"id":1,"name":"Bob","friend":null,"friendOf":null}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, indoc! { r#"
                mutation {
                    createOneUser(data: { id: 2, name: "Alice", friend: {connect:{id: 1}}}) {
                        id
                        name
                        friend { name }
                        friendOf { name }
                    }
                }
            "#}),
            @r###"{"data":{"createOneUser":{"id":2,"name":"Alice","friend":{"name":"Bob"},"friendOf":null}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, indoc! { r#"
                query {
                    findManyUser(where: { friend: { is: null }}){
                        id
                        name
                        friend { name }
                        friendOf { name }
                    }
                }
            "#}),
            @r###"{"data":{"findManyUser":[{"id":1,"name":"Bob","friend":null,"friendOf":{"name":"Alice"}}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, indoc! { r#"
                query {
                    findManyUser(where: { friendOf: { is: null }}){
                        id
                        name
                        friend { name }
                        friendOf { name }
                    }
                }
            "#}),
            @r###"{"data":{"findManyUser":[{"id":2,"name":"Alice","friend":{"name":"Bob"},"friendOf":null}]}}"###
        );

        Ok(())
    }
}
