use indoc::indoc;
use query_engine_tests::*;

// Note: Raw port without alternations from Scala.
#[test_suite(schema(schema))]
mod extended_relation_filters {
    fn schema() -> String {
        let schema = indoc! { "
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
                Artist   Artist  @relation(fields: [ArtistId], references: [id])
                Tracks   Track[]

                @@index([ArtistId])
            }

            model Genre {
                #id(id, String, @id, @default(cuid()))
                GenreId Int    @unique
                Name    String
                Tracks  Track[]
            }

            model MediaType {
                #id(id, String, @id, @default(cuid()))
                MediaTypeId Int    @unique
                Name        String
                Tracks      Track[]
            }

            model Track {
                #id(id, String, @id, @default(cuid()))
                TrackId      Int       @unique
                Name         String
                Composer     String
                Milliseconds Int
                Bytes        Int
                UnitPrice    Float
                AlbumId      String
                MediaTypeId  String
                GenreId      String

                Album        Album     @relation(fields: [AlbumId], references: [id])
                MediaType    MediaType @relation(fields: [MediaTypeId], references: [id])
                Genre        Genre     @relation(fields: [GenreId], references: [id])

                @@index([AlbumId])
                @@index([MediaTypeId])
                @@index([GenreId])
            }
        "};

        schema.to_owned()
    }

    #[connector_test]
    async fn basic_scalar_filter(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyArtist(where: { ArtistId: { equals: 1 }}){ Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn relation_filter_level1_depth1(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyAlbum(where: { Artist: { is: { Name: { equals: "CompleteArtist" }}}}){ AlbumId }}"#),
          @r###"{"data":{"findManyAlbum":[{"AlbumId":1}]}}"###
        );

        Ok(())
    }

    //   // MySql is case insensitive and Postgres case sensitive

    //   "MySql 1 level m-relation filter" should "work for `every`, `some` and `none`" taggedAs (IgnorePostgres, IgnoreMongo) in {
    //     server.query(query = """{artists(where:{Albums: { some: { Title: { startsWith: "album" }}}}){ Name }}""", project = project).toString should be(
    //       """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    //     server.query(query = """{artists(where:{Albums: { some: {Title: { startsWith: "t" }}}}){Name}}""", project = project).toString should be(
    //       """{"data":{"artists":[{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}""")

    //     server.query(query = """{artists(where:{Albums: { every: { Title: { contains: "album" }}}}){Name}}""", project = project).toString should be(
    //       """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    //     server.query(query = """{artists(where:{Albums: { every: { Title: { not: { contains: "the" }}}}}){Name}}""", project = project).toString should be(
    //       """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    //     server.query(query = """{artists(where:{Albums: { none: {Title: { contains: "the" }}}}){Name}}""", project = project).toString should be(
    //       """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    //     server.query(query = """{artists(where:{Albums: { none:{Title: { contains: "album" }}}}){Name}}""", project = project).toString should be(
    //       """{"data":{"artists":[{"Name":"ArtistWithoutAlbums"}]}}""")
    //   }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation {createOneGenre(data: {Name: "Genre1", GenreId: 1}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneGenre(data: {Name: "Genre2", GenreId: 2}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneGenre(data: {Name: "Genre3", GenreId: 3}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneGenre(data: {Name: "GenreThatIsNotUsed", GenreId: 4}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneMediaType(data: {Name: "MediaType1", MediaTypeId: 1}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneMediaType(data: {Name: "MediaType2", MediaTypeId: 2}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneMediaType(data: {Name: "MediaType3", MediaTypeId: 3}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation {createOneMediaType(data: {Name: "MediaTypeThatIsNotUsed", MediaTypeId: 4}){Name}}"#)
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation completeArtist {
                    createOneArtist(
                    data: {
                        Name: "CompleteArtist"
                        ArtistId: 1
                        Albums: {
                        create: [
                            {
                            Title: "Album1"
                            AlbumId: 1
                            Tracks: {
                                create: [
                                {
                                    Name: "Track1"
                                    TrackId: 1
                                    Composer: "Composer1"
                                    Milliseconds: 10000
                                    Bytes: 512
                                    UnitPrice: 1.51
                                    Genre: { connect: { GenreId: 1 } }
                                    MediaType: { connect: { MediaTypeId: 1 } }
                                }
                                ]
                            }
                            }
                        ]
                        }
                    }
                    ) {
                        Name
                    }
                }
            "#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation artistWithoutAlbums {
                    createOneArtist(data: { Name: "ArtistWithoutAlbums", ArtistId: 2 }) {
                        Name
                    }
                }
              "#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                    mutation artistWithAlbumButWithoutTracks {
                        createOneArtist(
                        data: {
                            Name: "ArtistWithOneAlbumWithoutTracks"
                            ArtistId: 3
                            Albums: { create: [{ Title: "TheAlbumWithoutTracks", AlbumId: 2 }] }
                        }
                        ) {
                            Name
                        }
                    }
                "#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation completeArtist2 {
                    createOneArtist(
                    data: {
                        Name: "CompleteArtist2"
                        ArtistId: 4
                        Albums: {
                        create: [
                            {
                            Title: "Album3"
                            AlbumId: 3
                            Tracks: {
                                create: [
                                {
                                    Name: "Track2"
                                    TrackId: 2
                                    Composer: "Composer1"
                                    Milliseconds: 11000
                                    Bytes: 1024
                                    UnitPrice: 2.51
                                    Genre: { connect: { GenreId: 2 } }
                                    MediaType: { connect: { MediaTypeId: 2 } }
                                }
                                {
                                    Name: "Track3"
                                    TrackId: 3
                                    Composer: "Composer2"
                                    Milliseconds: 9000
                                    Bytes: 24
                                    UnitPrice: 5.51
                                    Genre: { connect: { GenreId: 3 } }
                                    MediaType: { connect: { MediaTypeId: 3 } }
                                }
                                ]
                            }
                            }
                        ]
                        }
                    }
                    ) {
                        Name
                    }
                }
            "#
            })
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation completeArtist3 {
                    createOneArtist(
                    data: {
                        Name: "CompleteArtistWith2Albums"
                        ArtistId: 5
                        Albums: {
                        create: [
                            {
                            Title: "Album4"
                            AlbumId: 4
                            Tracks: {
                                create: [
                                {
                                    Name: "Track4"
                                    TrackId: 4
                                    Composer: "Composer1"
                                    Milliseconds: 15000
                                    Bytes: 10024
                                    UnitPrice: 12.51
                                    Genre: { connect: { GenreId: 1 } }
                                    MediaType: { connect: { MediaTypeId: 1 } }
                                }
                                {
                                    Name: "Track5"
                                    TrackId: 5
                                    Composer: "Composer2"
                                    Milliseconds: 19000
                                    Bytes: 240
                                    UnitPrice: 0.51
                                    Genre: { connect: { GenreId: 1 } }
                                    MediaType: { connect: { MediaTypeId: 1 } }
                                }
                                ]
                            }
                            }
                            {
                            Title: "Album5"
                            AlbumId: 5
                            Tracks: {
                                create: [
                                {
                                    Name: "Track6"
                                    TrackId: 6
                                    Composer: "Composer1"
                                    Milliseconds: 100
                                    Bytes: 724
                                    UnitPrice: 31.51
                                    Genre: { connect: { GenreId: 2 } }
                                    MediaType: { connect: { MediaTypeId: 3 } }
                                }
                                {
                                    Name: "Track7"
                                    TrackId: 7
                                    Composer: "Composer3"
                                    Milliseconds: 100
                                    Bytes: 2400
                                    UnitPrice: 5.51
                                    Genre: { connect: { GenreId: 1 } }
                                    MediaType: { connect: { MediaTypeId: 1 } }
                                }
                                ]
                            }
                            }
                        ]
                        }
                    }
                    ) {
                        Name
                    }
                }
            "#
            })
            .await?
            .assert_success();

        Ok(())
    }
}
