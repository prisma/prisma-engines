use query_engine_tests::*;

// TODO(dom): Not working on mongo
#[test_suite(schema(schema), exclude(MongoDb))]
mod order_by_aggr {
    use indoc::indoc;
    use query_engine_tests::{assert_query_many, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name  String
              posts Post[]
              #m2m(categories, Category[], Int)
            }
            
            model Post {
              #id(id, Int, @id)
              title  String
              user   User   @relation(fields: [userId], references: [id])
              userId Int
              #m2m(categories, Category[], Int)
            }
            
            model Category {
              #id(id, Int, @id)
              name   String
              #m2m(posts, Post[], Int)
              #m2m(users, User[], Int)
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn one2m_count_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: { posts: { _count: asc } }) {
              id
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":3,"posts":[]},{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: { posts: { _count: desc } }) {
              id
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"posts":[{"title":"alice_post_1"}]},{"id":3,"posts":[]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn m2m_count_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: { categories: { _count: asc } }) {
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_2","categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn m2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: { categories: { _count: desc } }) {
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"title":"bob_post_2","categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_1","categories":[{"name":"Finance"}]}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one2m_count_asc_field_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: [{ posts: { _count: asc } }, { name: asc }]) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":3,"name":"Motongo","posts":[]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo] Ordering by one2m count asc + field desc" should "work"
    #[connector_test]
    async fn one2m_count_asc_field_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: [{ name: desc }, { posts: { _count: asc } }]) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":3,"name":"Motongo","posts":[]},{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]},{"id":1,"name":"Alice","posts":[{"title":"alice_post_1"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo] Ordering by m2m count asc + field desc" should "work"
    #[connector_test]
    async fn m2m_count_asc_field_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc } }, { title: asc }]) {
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"title":"bob_post_1","categories":[{"name":"Finance"}]},{"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"title":"bob_post_2","categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo] Ordering by one2m field asc + m2m count desc" should "work"
    #[connector_test]
    async fn one2m_field_asc_m2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ user: { name: asc }}, { categories: { _count: desc }}]) {
              user {
                name
              }
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"user":{"name":"Alice"},"categories":[{"name":"News"},{"name":"Society"}]},{"user":{"name":"Bob"},"categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]},{"user":{"name":"Bob"},"categories":[{"name":"Finance"}]}]}}"###
        );

        Ok(())
    }

    // "[2+ Hops] Ordering by m2one2m count asc" should "work"
    #[connector_test]
    async fn m2one2m_count_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ user: { categories: { _count: asc } } }, { id: asc }]) {
              id
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"user":{"categories":[{"name":"Startup"}]}},{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[2+ Hops] Ordering by m2one2m count desc" should "work"
    #[connector_test]
    async fn m2one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        assert_query_many!(
            runner,
            r#"{
              findManyPost(orderBy: { user: { categories: { _count: desc } } }) {
                id
                user { categories { name } }
              }
            }"#,
            vec![
                r#"{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}"#,
                r#"{"data":{"findManyPost":[{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"user":{"categories":[{"name":"Startup"}]}}]}}"#,
            ]
        );

        Ok(())
    }

    // "[Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn m2m_count_asc_m2one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc }}, { user: { categories: { _count: desc }} }]) {
              id
              categories(orderBy: { name: asc }) {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}]},{"id":1,"categories":[{"name":"News"},{"name":"Society"}]},{"id":3,"categories":[{"name":"Gaming"},{"name":"Hacking"},{"name":"History"}]}]}}"###
        );

        Ok(())
    }

    // "[Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn m2one_field_asc_m2one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        assert_query_many!(
            runner,
            r#"{
              findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { _count: desc }} }]) {
                id
                user {
                  name
                  categories { name }
                }
              }
            }"#,
            vec![
                r#"{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"#,
                r#"{"data":{"findManyPost":[{"id":1,"user":{"name":"Alice","categories":[{"name":"Startup"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"#,
            ]
        );

        Ok(())
    }

    // With pagination tests

    // "[Cursor] Ordering by one2m count asc" should "work"
    #[connector_test]
    async fn cursor_one2m_count_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: { posts: { _count: asc } }, cursor: { id: 2 }) {
              id
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor] Ordering by one2m count desc" should "work"
    #[connector_test]
    async fn cursor_one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: { posts: { _count: desc } }, cursor: { id: 1 }) {
              id
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":1,"posts":[{"title":"alice_post_1"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor] Ordering by m2m count asc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: { categories: { _count: asc } }, cursor: { id: 2 }, take: 2) {
              id
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor] Ordering by m2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: { categories: { _count: desc } }, cursor: { id: 1 }, take: 2) {
              id
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]},{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by one2m count asc + field asc"
    #[connector_test]
    async fn cursor_one2m_count_asc_field_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: [{ posts: { _count: asc } }, { name: asc }], cursor: { id: 2 }) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by one2m count asc + field desc" should "work"
    #[connector_test]
    async fn cursor_one2m_count_asc_field_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyUser(orderBy: [{ name: desc }, { posts: { _count: asc } }], cursor: { id: 2 }, take: 1) {
              id
              name
              posts {
                title
              }
            }
          }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"name":"Bob","posts":[{"title":"bob_post_1"},{"title":"bob_post_2"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by m2m count asc + field desc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_asc_field_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc } }, { title: asc }], cursor: { id: 2 }, take: 2) {
              id
              title
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"title":"bob_post_1","categories":[{"name":"Finance"}]},{"id":1,"title":"alice_post_1","categories":[{"name":"News"},{"name":"Society"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo] Ordering by one2m field asc + m2m count desc" should "work"
    #[connector_test]
    async fn cursor_one2m_field_asc_m2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ user: { name: asc }}, { categories: { _count: desc }}], cursor: { id: 1 }, take: 2) {
              id
              title
              user {
                name
              }
              categories {
                name
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":1,"title":"alice_post_1","user":{"name":"Alice"},"categories":[{"name":"News"},{"name":"Society"}]},{"id":3,"title":"bob_post_2","user":{"name":"Bob"},"categories":[{"name":"History"},{"name":"Gaming"},{"name":"Hacking"}]}]}}"###
        );

        Ok(())
    }

    // "[Cursor][2+ Hops] Ordering by m2one2m count asc" should "work"
    #[connector_test]
    async fn cursor_m2one2m_count_asc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ user: { categories: { _count: asc } } }, { id: asc }], cursor: { id: 2 }, take: 1) {
              id
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][2+ Hops] Ordering by m2one2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ user: { categories: { _count: desc } } }, { id: asc }], cursor: { id: 2 }, take: 2) {
              id
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo][2+ Hops] Ordering by m2m count asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2m_count_asc_m2one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ categories: { _count: asc }}, { user: { categories: { _count: desc }} }], cursor: { id: 2 }, take: 2) {
              id
              categories { name }
              user { categories { name } }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"categories":[{"name":"Finance"}],"user":{"categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":1,"categories":[{"name":"News"},{"name":"Society"}],"user":{"categories":[{"name":"Startup"}]}}]}}"###
        );

        Ok(())
    }

    // "[Cursor][Combo][2+ Hops] Ordering by m2one field asc + m2one2m count desc" should "work"
    #[connector_test]
    async fn cursor_m2one_field_asc_m2one2m_count_desc(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{
            findManyPost(orderBy: [{ user: { name: asc }}, { user: { categories: { _count: desc }} }, { id: asc }], cursor: { id: 2 }, take: 2) {
              id
              user {
                name
                categories { name }
              }
            }
          }"#),
          @r###"{"data":{"findManyPost":[{"id":2,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}},{"id":3,"user":{"name":"Bob","categories":[{"name":"Computer Science"},{"name":"Music"}]}}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, name: "Alice", categories: { create: [{ id: 1, name: "Startup" }] }, posts: { create: { id: 1, title: "alice_post_1", categories: { create: [{ id: 2, name: "News" }, { id: 3, name: "Society" }] }} } }"#).await?;
        create_row(runner, r#"{ id: 2, name: "Bob", categories: { create: [{ id: 4, name: "Computer Science" }, { id: 5, name: "Music" }] }, posts: { create: [{ id: 2, title: "bob_post_1", categories: { create: [{ id: 6, name: "Finance" }] } }, { id: 3, title: "bob_post_2", categories: { create: [{ id: 7, name: "History" }, { id: 8, name: "Gaming" }, { id: 9, name: "Hacking" }] } }] } }"#).await?;
        create_row(runner, r#"{ id: 3, name: "Motongo" }"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneUser(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}
