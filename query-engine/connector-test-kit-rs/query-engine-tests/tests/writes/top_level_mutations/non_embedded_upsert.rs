use query_engine_tests::*;

#[test_suite(schema(dm_p1_to_c1))]
mod non_embedded_upsert {
    use indoc::indoc;
    use query_engine_tests::{run_query, run_query_json};

    fn dm_p1_to_c1() -> String {
        let schema = indoc! {
            r#"model List{
              #id(id, Int, @id)
              uList  String? @unique
              todoId Int?

              todo  Todo?   @relation(fields: [todoId], references: [id])
           }

           model Todo {
              #id(id, Int, @id)
              uTodo String? @unique
              list  List?
           }"#
        };

        schema.to_owned()
    }

    fn dm_pm_to_cm() -> String {
        let schema = indoc! {
            r#"model List{
              #id(id, Int, @id)
              uList String? @unique
              #m2m(todoes, Todo[], Int)
           }

           model Todo{
              #id(id, Int, @id)
              uTodo String? @unique
              #m2m(lists, List[], Int)
              #m2m(tags, Tag[], Int)
           }

           model Tag{
              #id(id, Int, @id)
              uTag String @unique
              #m2m(todoes, Todo[], Int)
           }"#
        };

        schema.to_owned()
    }

    // "An upsert on the top level" should "execute a nested connect in the create branch"
    #[connector_test]
    async fn nested_connect_in_create(runner: Runner) -> TestResult<()> {
        // Seed data
        run_query!(&runner, r#"mutation{createOneTodo(data:{id: 1, uTodo: "B"}){uTodo}}"#);
        run_query!(
            &runner,
            r#"mutation {upsertOneList(
            where:{uList: "Does not Exist"}
            create:{id: 1, uList:"A" todo: {connect: {uTodo: "B"}}}
            update:{todo: {connect: {uTodo: "Should not matter"}}}
          ){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todo":{"uTodo":"B"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTodo { uTodo } }"#),
          @r###"{"data":{"findManyTodo":[{"uTodo":"B"}]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 1);

        Ok(())
    }

    // "An upsert on the top level" should "execute a nested connect in the update branch"
    #[connector_test]
    async fn nested_connect_in_update(runner: Runner) -> TestResult<()> {
        // Seed data
        run_query!(&runner, r#"mutation{createOneTodo(data:{id: 1, uTodo: "B"}){uTodo}}"#);
        run_query!(&runner, r#"mutation{createOneList(data:{id: 1, uList:"A"}){uList}}"#);
        run_query!(
            &runner,
            r#"mutation {upsertOneList(
                where:{uList: "A"}
                create:{id: 2, uList:"A" todo: {connect: {uTodo: "Should not Matter"}}}
                update:{todo: {connect: {uTodo: "B"}}}
            ){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todo":{"uTodo":"B"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyTodo {uTodo}}"#),
          @r###"{"data":{"findManyTodo":[{"uTodo":"B"}]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 1);

        Ok(())
    }

    // "An upsert on the top level" should "execute a nested disconnect in the update branch"
    #[connector_test]
    async fn nested_disconnect_in_update(runner: Runner) -> TestResult<()> {
        // Seed data
        run_query!(
            &runner,
            r#"mutation{createOneTodo(data:{id: 1, uTodo: "B", list: {create: {id: 1, uList:"A"}}}){uTodo}}"#
        );
        run_query!(
            &runner,
            r#"mutation {upsertOneList(
          where:{uList: "A"}
          create:{id: 2, uList:"A" todo: {connect: {uTodo: "Should not Matter"}}}
          update:{todo: {disconnect: true}}
        ){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todo":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyTodo {uTodo}}"#),
          @r###"{"data":{"findManyTodo":[{"uTodo":"B"}]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 1);

        Ok(())
    }

    // "An upsert on the top level" should "execute a nested delete in the update branch"
    #[connector_test(exclude(SqlServer))]
    async fn nested_delete_in_update(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation{createOneTodo(data:{id: 1, uTodo: "B", list: {create: {id: 1, uList:"A"}}}){uTodo}}"#
        );
        run_query!(
            &runner,
            r#"mutation {upsertOneList(
          where:{uList: "A"}
          create:{id: 2, uList:"A" todo: {connect: {uTodo: "Should not Matter"}}}
          update:{todo: {delete: true}}
        ){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todo":null}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyTodo {uTodo}}"#),
          @r###"{"data":{"findManyTodo":[]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 0);

        Ok(())
    }

    // "An upsert on the top level" should "only execute the nested create mutations of the correct update branch"
    #[connector_test]
    async fn execute_nested_create_of_correct_branch(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation {createOneList(data: {id:1, uList: "A"}){id}}"#);
        run_query!(
            &runner,
            r#"mutation {upsertOneList(
                where:{uList: "A"}
                create:{id: 2, uList:"B"  todo: {create: {id: 1, uTodo: "B"}}}
                update:{uList:"C"  todo: {create: {id: 2, uTodo: "C"}}}
            ){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todo {uTodo}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"C","todo":{"uTodo":"C"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyTodo {uTodo}}"#),
          @r###"{"data":{"findManyTodo":[{"uTodo":"C"}]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 1);

        Ok(())
    }

    // "A nested upsert" should "execute the nested connect mutations of the correct create branch"
    #[connector_test(schema(dm_pm_to_cm))]
    async fn nested_connect_in_correct_create_branch(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation {createOneTag(data:{id: 1, uTag: "D"}){uTag}}"#);
        run_query!(&runner, r#"mutation {createOneList(data:{id: 1, uList: "A"}){id}}"#);
        run_query!(
            &runner,
            r#"mutation{updateOneList(
                where:{uList: "A"}
                data:{todoes: {
                upsert:{
                      where:{uTodo: "B"}
                      create:{id:1, uTodo:"C" tags: {connect: {uTag: "D"}}}
                      update:{uTodo:{set:"Should Not Matter"},tags: {create: {id: 2, uTag: "D"}}}
                }}
              }){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todoes {uTodo, tags {uTag }}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todoes":[{"uTodo":"C","tags":[{"uTag":"D"}]}]}]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 1);
        assert_eq!(count_items(&runner, "findManyTag").await?, 1);

        Ok(())
    }

    // "A nested upsert" should "execute the nested connect mutations of the correct update branch"
    #[connector_test(schema(dm_pm_to_cm))]
    async fn nested_connect_in_correct_update_branch(runner: Runner) -> TestResult<()> {
        // Seed data
        run_query!(&runner, r#"mutation { createOneTag(data:{id: 1, uTag: "D"}){uTag}}"#);
        run_query!(
            &runner,
            r#"mutation {createOneList(data: {id: 1, uList: "A" todoes: {create: {id: 1, uTodo: "B"}}}){id}}"#
        );
        run_query!(
            &runner,
            r#"mutation{updateOneList(
                where:{uList: "A"}
                data:{todoes: {
                  upsert:{
                        where:{uTodo: "B"}
                        create:{id: 1, uTodo:"Should Not Matter" tags: {connect: {uTag: "D"}}}
                        update:{uTodo: { set: "C" }, tags: { connect: { uTag: "D" }}}
                  }}
              }){id}}"#
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findManyList {uList, todoes {uTodo, tags {uTag }}}}"#),
          @r###"{"data":{"findManyList":[{"uList":"A","todoes":[{"uTodo":"C","tags":[{"uTag":"D"}]}]}]}}"###
        );

        assert_eq!(count_items(&runner, "findManyList").await?, 1);
        assert_eq!(count_items(&runner, "findManyTodo").await?, 1);
        assert_eq!(count_items(&runner, "findManyTag").await?, 1);

        Ok(())
    }

    async fn count_items(runner: &Runner, name: &str) -> TestResult<usize> {
        let res = run_query_json!(runner, format!("query {{ {} {{ id }} }}", name));
        let count = &res["data"][name];

        match count {
            serde_json::Value::Array(array) => Ok(array.len()),
            _ => unreachable!(),
        }
    }
}
