use query_engine_tests::*;

#[test_suite(schema(schema))]
mod singlular_batch {
    use indoc::indoc;

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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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

        let batch_results = runner.batch(queries, false).await?;
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
            r#"query { findUniqueArtist(where: { ArtistId: 1}) { Name }}"#.to_string(),
            r#"query { findUniqueArtist(where: { ArtistId: 1}) { Name }}"#.to_string(),
        ];

        let batch_results = runner.batch(queries, false).await?;
        insta::assert_snapshot!(
            batch_results.to_string(),
            @r###"{"batchResult":[{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"findUniqueArtist":{"Name":"ArtistWithoutAlbums"}}}]}"###
        );

        Ok(())
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
