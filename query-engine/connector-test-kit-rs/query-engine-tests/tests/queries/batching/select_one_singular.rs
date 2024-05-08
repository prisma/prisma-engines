use query_engine_tests::*;

#[test_suite(schema(schema))]
mod singular_batch {
    use indoc::indoc;
    use query_engine_tests::{
        query_core::{BatchDocument, QueryDocument},
        run_query, Runner, TestResult,
    };

    fn schema() -> String {
        let schema = indoc! {
            r#"
                model Artist {
                    #id(id, String, @id, @default(cuid()))
                    ArtistId Int     @unique
                    Name     String
                    Albums   Album[]
                }

                model Album {
                    #id(id, String, @id, @default(cuid()))
                    AlbumId  Int     @unique
                    Title    String
                    ArtistId String

                    Artist  Artist  @relation(fields: [ArtistId], references: [id])
                    @@index([ArtistId])
                }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn one_success(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![r#"query { findUniqueArtist(where: { ArtistId: 1 }){ Name }}"#.to_string()];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}}]}"###
        );

        // With non-unique filter
        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 1, Name: "ArtistWithoutAlbums" }){ Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_success_one_fail(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { Name, ArtistId }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420 }) { Name, ArtistId }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 2 }) { ArtistId, Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums","ArtistId":1}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"Name":"ArtistWithOneAlbumWithoutTracks","ArtistId":2}}}]}"###
        );

        // With non-unique filters
        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 1, Name: "ArtistWithoutAlbums" }) { Name, ArtistId }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420, Name: "Bonamassa" }) { Name, ArtistId }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 2, Name: { equals: "ArtistWithOneAlbumWithoutTracks" } }) { ArtistId, Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums","ArtistId":1}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"Name":"ArtistWithOneAlbumWithoutTracks","ArtistId":2}}}]}"###
        );

        Ok(())
    }

    // "Two successful queries and one failing with different selection set" should "work"
    #[connector_test]
    async fn two_success_one_fail_diff_set(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { ArtistId, Name }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420 }) { Name }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 2 }) { Name, ArtistId }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"ArtistId":1,"Name":"ArtistWithoutAlbums"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"Name":"ArtistWithOneAlbumWithoutTracks","ArtistId":2}}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_traversal(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 2 }) { Albums { AlbumId, Title }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { Albums { Title, AlbumId }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420 }) { Albums { AlbumId, Title }}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findUniqueArtist":{"Albums":[]}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        // With non-unique filter
        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 2, Name: "ArtistWithOneAlbumWithoutTracks" }) { Albums { AlbumId, Title }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1, Name: "ArtistWithoutAlbums" }) { Albums { Title, AlbumId }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420, Name: "Bonamassa" }) { Albums { AlbumId, Title }}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findUniqueArtist":{"Albums":[]}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_traversal_filtered(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 2 }) { Albums(where: { AlbumId: { equals: 2 }}) { AlbumId, Title }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { Albums(where: { AlbumId: { equals: 2 }}) { Title, AlbumId }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420 }) { Albums(where: { AlbumId: { equals: 2 }}) { AlbumId, Title }}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findUniqueArtist":{"Albums":[]}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        // With non-unique filter
        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 2, Name: "ArtistWithOneAlbumWithoutTracks" }) { Albums(where: { AlbumId: { equals: 2 }}) { AlbumId, Title }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1, Name: "ArtistWithoutAlbums" }) { Albums(where: { AlbumId: { equals: 2 }}) { Title, AlbumId }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420, Name: "Bonamassa" }) { Albums(where: { AlbumId: { equals: 2 }}) { AlbumId, Title }}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findUniqueArtist":{"Albums":[]}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_traversal_filtered_diff(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 2 }) { Albums(where: { AlbumId: { equals: 2 }}) { AlbumId, Title }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { Albums(where: { AlbumId: { equals: 1 }}) { Title, AlbumId }}}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420 }) { Albums(where: { AlbumId: { equals: 2 }}) { AlbumId, Title }}}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findUniqueArtist":{"Albums":[]}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn one_failure(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![r#"query { findUniqueArtist(where: { ArtistId: 420 }) { Name }}"#.to_string()];

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
            r#"query { findUniqueArtist(where: { ArtistId: 1}) { Name }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 420}) { Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"findUniqueArtist":null}}]}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn two_equal_queries(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { Name }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1 }) { Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}}]}"###
        );

        // With non unique filters
        let queries = vec![
            r#"query { findUniqueArtist(where: { ArtistId: 1, Name: "ArtistWithoutAlbums" }) { Name }}"#.to_string(),
            r#"query { findUniqueArtist(where: { Name: "ArtistWithoutAlbums", ArtistId: 1 }) { Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false, None).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}}]}"###
        );

        Ok(())
    }

    fn bigint_id() -> String {
        let schema = indoc! {
            r#" model TestModel {
                #id(id, BigInt, @id)
            }"#
        };

        schema.to_owned()
    }

    // Regression test for https://github.com/prisma/prisma/issues/18096
    #[connector_test(schema(bigint_id))]
    async fn batch_bigint_id(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id } }"#);
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 2 }) { id } }"#);
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 3 }) { id } }"#);

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let queries = vec![
                    r#"query { findUniqueTestModel(where: { id: 1 }) { id }}"#.to_string(),
                    r#"query { findUniqueTestModel(where: { id: 2 }) { id }}"#.to_string(),
                    r#"query { findUniqueTestModel(where: { id: 3 }) { id }}"#.to_string(),
                ];

                let batch_results = runner.batch(queries, false, None).await?;

                insta::assert_snapshot!(
                    batch_results.to_string(),
                    @r###"{"batchResult":[{"data":{"findUniqueTestModel":{"id":"1"}}},{"data":{"findUniqueTestModel":{"id":"2"}}},{"data":{"findUniqueTestModel":{"id":"3"}}}]}"###
                );
            }
            EngineProtocol::Json => {
                let queries = vec![
                    r#"{ "modelName": "TestModel", "action": "findUnique", "query": { "arguments": { "where": { "id": { "$type": "BigInt", "value": "1" } } }, "selection": { "$scalars": true } } }"#.to_string(),
                    r#"{ "modelName": "TestModel", "action": "findUnique", "query": { "arguments": { "where": { "id": { "$type": "BigInt", "value": "2" } } }, "selection": { "$scalars": true } } }"#.to_string(),
                    r#"{ "modelName": "TestModel", "action": "findUnique", "query": { "arguments": { "where": { "id": { "$type": "BigInt", "value": "3" } } }, "selection": { "$scalars": true } } }"#.to_string(),
                ];

                let batch_results = runner.batch_json(queries, false, None).await?;

                insta::assert_snapshot!(
                    batch_results.to_string(),
                    @r###"{"batchResult":[{"data":{"findUniqueTestModel":{"id":{"$type":"BigInt","value":"1"}}}},{"data":{"findUniqueTestModel":{"id":{"$type":"BigInt","value":"2"}}}},{"data":{"findUniqueTestModel":{"id":{"$type":"BigInt","value":"3"}}}}]}"###
                );
            }
        }

        Ok(())
    }

    fn enum_id() -> String {
        let schema = indoc! {
            r#"
            enum IdEnum {
                A
                B
            }

            model TestModel {
                #id(id, IdEnum, @id)
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(enum_id), capabilities(Enums))]
    async fn batch_enum(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: "A" }) { id } }"#);
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: "B" }) { id } }"#);

        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueTestModel(where: { id: "A" }) { id } }"#.to_string(),
                r#"{ findUniqueTestModel(where: { id: "B" }) { id } }"#.to_string(),
            ],
        )
        .await?;

        insta::assert_snapshot!(
          res.to_string(),
          @r###"{"batchResult":[{"data":{"findUniqueTestModel":{"id":"A"}}},{"data":{"findUniqueTestModel":{"id":"B"}}}]}"###
        );
        assert!(compact_doc.is_compact());

        Ok(())
    }

    fn boolean_unique() -> String {
        let schema = indoc! {
            r#"
        model User {
            #id(id, String, @id)
            isManager Boolean? @unique
          }
        "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(boolean_unique))]
    async fn batch_boolean(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
            createOneUser(data: { id: "A", isManager: true }) { id }
        }"#
        );
        run_query!(
            &runner,
            r#"mutation {
            createOneUser(data: { id: "B", isManager: false }) { id }
        }"#
        );

        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueUser(where: { isManager: true }) { id, isManager } }"#.to_string(),
                r#"{ findUniqueUser(where: { isManager: false }) { id, isManager } }"#.to_string(),
            ],
        )
        .await?;

        insta::assert_snapshot!(
            res.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueUser":{"id":"A","isManager":true}}},{"data":{"findUniqueUser":{"id":"B","isManager":false}}}]}"###
        );

        assert!(compact_doc.is_compact());

        Ok(())
    }

    // Regression test for https://github.com/prisma/prisma/issues/16548
    #[connector_test(schema(schemas::generic))]
    async fn repro_16548(runner: Runner) -> TestResult<()> {
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id } }"#);
        run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 2 }) { id } }"#);

        // Working case
        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueTestModelOrThrow(where: { id: 1 }) { id } }"#.to_string(),
                r#"{ findUniqueTestModelOrThrow(where: { id: 2 }) { id } }"#.to_string(),
            ],
        )
        .await?;
        insta::assert_snapshot!(
            res.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueTestModelOrThrow":{"id":1}}},{"data":{"findUniqueTestModelOrThrow":{"id":2}}}]}"###
        );
        assert!(compact_doc.is_compact());

        // Failing case
        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueTestModelOrThrow(where: { id: 2 }) { id } }"#.to_string(),
                r#"{ findUniqueTestModelOrThrow(where: { id: 3 }) { id } }"#.to_string(),
            ],
        )
        .await?;
        insta::assert_snapshot!(
          res.to_string(),
          @r###"{"batchResult":[{"data":{"findUniqueTestModelOrThrow":{"id":2}}},{"errors":[{"error":"An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.","user_facing_error":{"is_panic":false,"message":"An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.","meta":{"cause":"Expected a record, found none."},"error_code":"P2025"}}]}]}"###
        );
        assert!(compact_doc.is_compact());

        // Mix of findUnique & findUniqueOrThrow
        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueTestModel(where: { id: 3 }) { id } }"#.to_string(),
                r#"{ findUniqueTestModelOrThrow(where: { id: 2 }) { id } }"#.to_string(),
            ],
        )
        .await?;
        insta::assert_snapshot!(
          res.to_string(),
          @r###"{"batchResult":[{"data":{"findUniqueTestModel":null}},{"data":{"findUniqueTestModelOrThrow":{"id":2}}}]}"###
        );
        assert!(!compact_doc.is_compact());

        // Mix of findUnique & findUniqueOrThrow
        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueTestModel(where: { id: 2 }) { id } }"#.to_string(),
                r#"{ findUniqueTestModelOrThrow(where: { id: 4 }) { id } }"#.to_string(),
            ],
        )
        .await?;
        insta::assert_snapshot!(
          res.to_string(),
          @r###"{"batchResult":[{"data":{"findUniqueTestModel":{"id":2}}},{"errors":[{"error":"KnownError { message: \"An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.\", meta: Object {\"cause\": String(\"Expected a record, found none.\")}, error_code: \"P2025\" }","user_facing_error":{"is_panic":false,"message":"An operation failed because it depends on one or more records that were required but not found. Expected a record, found none.","meta":{"cause":"Expected a record, found none."},"error_code":"P2025"}}]}]}"###
        );
        assert!(!compact_doc.is_compact());

        // Mix of findUnique & findUniqueOrThrow
        let (res, compact_doc) = compact_batch(
            &runner,
            vec![
                r#"{ findUniqueTestModelOrThrow(where: { id: 2 }) { id } }"#.to_string(),
                r#"{ findUniqueTestModel(where: { id: 3 }) { id } }"#.to_string(),
            ],
        )
        .await?;
        insta::assert_snapshot!(
          res.to_string(),
          @r###"{"batchResult":[{"data":{"findUniqueTestModelOrThrow":{"id":2}}},{"data":{"findUniqueTestModel":null}}]}"###
        );
        assert!(!compact_doc.is_compact());

        Ok(())
    }

    fn citext_unique() -> String {
        let schema = indoc! { r#"
            model User {
                #id(id, String, @id)
                caseInsensitiveField String @unique @test.Citext
            }"#
        };

        schema.to_owned()
    }

    // Regression test for https://github.com/prisma/prisma/issues/13534
    #[connector_test(only(Postgres), schema(citext_unique), db_extensions("citext"))]
    async fn repro_13534(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
            createOneUser(data: { id: "9df0f936-51d6-4c55-8e01-5144e588a8a1", caseInsensitiveField: "hello world" }) { id }
        }"#
        );

        let queries = vec![
            r#"{ findUniqueUser(
                where: { caseInsensitiveField: "HELLO WORLD" }
            ) { id, caseInsensitiveField } }"#
                .to_string(),
            r#"{ findUniqueUser(
                where: { caseInsensitiveField: "HELLO WORLD" }
            ) { id, caseInsensitiveField } }"#
                .to_string(),
        ];

        let (res, compact_doc) = compact_batch(&runner, queries.clone()).await?;

        assert!(!compact_doc.is_compact());

        insta::assert_snapshot!(
            res.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueUser":{"id":"9df0f936-51d6-4c55-8e01-5144e588a8a1","caseInsensitiveField":"hello world"}}},{"data":{"findUniqueUser":{"id":"9df0f936-51d6-4c55-8e01-5144e588a8a1","caseInsensitiveField":"hello world"}}}]}"###
        );

        Ok(())
    }

    fn schema_uuid() -> String {
        let schema = indoc! { r#"
            model Cup {
                #id(id, String, @id, @default(uuid()), @map("id"), @test.Uuid)
            }
        "# };

        schema.to_owned()
    }

    #[connector_test(schema(schema_uuid))]
    async fn batch_uuid(runner: Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation {
                    createOneCup(data: { id: "11111111111111111111111111111111" })
                    { id }
                }"#,
            )
            .await?
            .assert_success();

        let queries = vec![
            r#"query {
                    findUniqueCup(where: { id: "11111111111111111111111111111111" })
                    { id }
                }"#
            .to_owned(),
            r#"query {
                    findUniqueCup(where: { id: "11111111111111111111111111111111" })
                    { id }
                }"#
            .to_owned(),
        ];

        let (res, compact_doc) = compact_batch(&runner, queries.clone()).await?;

        assert!(compact_doc.is_compact());

        insta::assert_snapshot!(
            res.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueCup":{"id":"11111111-1111-1111-1111-111111111111"}}},{"data":{"findUniqueCup":{"id":"11111111-1111-1111-1111-111111111111"}}}]}"###
        );

        Ok(())
    }

    async fn compact_batch(runner: &Runner, queries: Vec<String>) -> TestResult<(QueryResult, BatchDocument)> {
        let res = runner.batch(queries.clone(), false, None).await?;

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

        Ok((res, batch.compact(runner.query_schema())))
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(
                r#"mutation artistWithoutAlbums {
                createOneArtist(data: { Name: "ArtistWithoutAlbums", ArtistId: 1 }) {
                  Name
                }
              }
              "#,
            )
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation artistWithAlbumButWithoutTracks {
                createOneArtist(
                  data: {
                    Name: "ArtistWithOneAlbumWithoutTracks"
                    ArtistId: 2
                    Albums: { create: [{ Title: "TheAlbumWithoutTracks", AlbumId: 2 }] }
                  }
                ) {
                  Name
                }
              }
              "#,
            )
            .await?
            .assert_success();

        Ok(())
    }
}
