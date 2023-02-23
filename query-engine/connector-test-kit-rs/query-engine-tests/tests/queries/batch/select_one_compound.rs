use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(AnyId))]
mod compound_batch {
    use indoc::indoc;
    use query_engine_tests::query_core::{BatchDocument, QueryDocument};

    fn schema() -> String {
        let schema = indoc! {
            r#"model Artist {
                firstName String
                lastName  String
                non_unique Int?

                @@unique([firstName, lastName])
              }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn one_success(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { firstName_lastName: { firstName:"Musti", lastName:"Naukio" }}) { firstName lastName }}"#.to_string()
        ];
        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}}]}"###
        );

        // With non unique filters
        let queries = vec![
            r#"query { findUniqueArtist(where: { firstName_lastName: { firstName:"Musti", lastName:"Naukio" }, non_unique: 0}) { firstName lastName non_unique }}"#.to_string()
        ];
        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio","non_unique":0}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_success_one_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {firstName lastName}}"#.to_string(),
        ];
        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        // With non unique filters
        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: 0}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: 1}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}, non_unique: null}) {firstName lastName}}"#.to_string(),
        ];
        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_success_sel_set_reorder(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {lastName firstName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        // With non unique filters
        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: 0}) {non_unique firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}, non_unique: null}) {lastName firstName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"non_unique":0,"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":{"lastName":"Musti","firstName":"Naukio"}}}]}"###
        );

        Ok(())
    }

    // "Two successful queries and one failing with different selection set" should "work"
    #[connector_test]
    async fn two_success_one_fail_diff_set(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
           r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
           r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {lastName}}"#.to_string(),
           r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        // With non unique filters
        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: { equals: 0 }}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: 1}) {lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}, non_unique: null}) {firstName lastName}}"#.to_string(),
         ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {lastName}}"#
                .to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_failure_one_success(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn no_compact_but_works_as_batch(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: { gte: 0 }}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}, non_unique: null}) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_equal_queries(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"} }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{lastName:"Naukio",firstName:"Musti"} }) {firstName lastName}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;

        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}}]}"###
        );

        Ok(())
    }

    fn should_batch_schema() -> String {
        let schema = indoc! {
            r#"model Artist {
                #id(id, Int, @id)
                firstName String
                lastName  String
                non_unique Int?

                songs Song[]

                @@unique([firstName, lastName])
              }

              model Song {
                #id(id, Int, @id)
                title String

                artistId Int?
                artist Artist? @relation(fields: [artistId], references: [id])
              }
              "#
        };

        schema.to_owned()
    }

    // Ensures non compactable batch are not compacted
    #[connector_test(schema(should_batch_schema))]
    async fn should_only_batch_if_possible(runner: Runner) -> TestResult<()> {
        // COMPACT: Queries use compound unique
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(doc.is_compact());

        // COMPACT: Queries use compound unique + non unique equal filter (shorthand syntax)
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: 0}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: 1}) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(doc.is_compact());

        // COMPACT: Queries use compound unique + non unique equal filter
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: 0}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: { equals: 1 }}) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(doc.is_compact());

        // COMPACT: Queries use compound unique + non unique equal filter
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: 0}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(doc.is_compact());

        // COMPACT: Queries use compound unique + non unique equal filter (null)
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: null}) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: { equals: null }}) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(doc.is_compact());

        // NO COMPACT: Queries use boolean operators
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, OR: [{ non_unique: 0 }] }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, OR: [{ non_unique: 0 }] }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: Queries use boolean operators
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, AND: [{ non_unique: 0 }] }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, AND: [{ non_unique: 0 }] }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: Queries use boolean operators
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, NOT: [{ non_unique: 0 }] }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, NOT: [{ non_unique: 1 }] }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: Queries use relation
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, songs: { some: { title: "Bohemian Rapsody" } } }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, songs: { some: { title: "Somebody To Love" } } }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: Queries use non unique filter that's not EQUALS
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}, non_unique: { gt: 1 } }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: { gt: 1 } }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: One of the query uses a non unique filter that's not EQUALS
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"} }) {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: { gt: 1 } }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: One of the query is not a findUnique
        let doc = compact_batch(&runner, vec![
            r#"query {findManyArtist {firstName lastName}}"#.to_string(),
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: { gt: 1 } }) {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        // NO COMPACT: One of the query is not a findUnique
        let doc = compact_batch(&runner, vec![
            r#"query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}, non_unique: 1 }) {firstName lastName}}"#.to_string(),
            r#"query {findManyArtist {firstName lastName}}"#.to_string(),
        ]).await?;
        assert!(!doc.is_compact());

        Ok(())
    }

    #[connector_test(schema(common_list_types), capabilities(ScalarLists))]
    async fn should_only_batch_if_possible_list(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1, int: [1, 2, 3] }) { id } }"#
        );
        run_query!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 2, int: [1, 3, 4] }) { id } }"#
        );

        let queries = vec![
            r#"query {findUniqueTestModel(where:{ id: 1, int: { equals: [1, 2, 3] } }) {id, int}}"#.to_string(),
            r#"query {findUniqueTestModel(where:{ id: 2, int: { equals: [1, 3, 4] } }) {id, int}}"#.to_string(),
        ];

        // COMPACT: Queries use scalar list
        let doc = compact_batch(&runner, queries.clone()).await?;
        assert!(doc.is_compact());

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueTestModel":{"id":1,"int":[1,2,3]}}},{"data":{"findUniqueTestModel":{"id":2,"int":[1,3,4]}}}]}"###
        );

        Ok(())
    }

    async fn compact_batch(runner: &Runner, queries: Vec<String>) -> TestResult<BatchDocument> {
        // Ensure individual queries are valid. Helps to debug tests when writing them.
        for q in queries.iter() {
            run_query!(runner, q.to_string());
        }

        // Ensure batched queries are valid
        runner.batch(queries.clone(), false, None).await?.assert_success();

        let doc = GraphqlBody::Multi(MultiQuery::new(
            queries.into_iter().map(Into::into).collect(),
            false,
            None,
        ))
        .into_doc()
        .unwrap();
        let batch = match doc {
            QueryDocument::Multi(batch) => batch.compact(runner.query_schema()),
            _ => unreachable!(),
        };

        Ok(batch.compact(runner.query_schema()))
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneArtist(data: { firstName: "Musti" lastName: "Naukio", non_unique: 0 }) { firstName }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneArtist(data: { firstName: "Naukio" lastName: "Musti" }) { firstName }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
