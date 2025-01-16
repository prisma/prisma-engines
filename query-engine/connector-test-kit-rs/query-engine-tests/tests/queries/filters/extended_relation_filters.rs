use query_engine_tests::*;

// Note: Raw port without alternations from Scala. Original name `ExtendedRelationFilterSpec.scala`.
#[test_suite(schema(schema))]
mod ext_rel_filters {
    use indoc::indoc;
    use query_engine_tests::run_query;

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
    async fn basic_scalar_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyArtist(where: { ArtistId: { equals: 1 }}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn rel_filter_l1_depth1(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Artist: { is: { Name: { equals: "CompleteArtist" }}}}) { AlbumId }}"#),
          @r###"{"data":{"findManyAlbum":[{"AlbumId":1}]}}"###
        );

        Ok(())
    }

    // MySql is case insensitive and Postgres case sensitive
    #[connector_test(only(MySQL))]
    async fn mysql_rel1_many_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Artist: { is: { Name: { equals: "CompleteArtist" }}}}) { AlbumId }}"#),
          @r###"{"data":{"findManyAlbum":[{"AlbumId":1}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Title: { startsWith: "album" }}}}) { Name }}"#),
            @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Title: { startsWith: "t" }}}}) { Name }}"#),
            @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Title: { contains: "album" }}}}) { Name }}"#),
            @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Title: { not: { contains: "the" }}}}}) { Name }}"#),
            @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyArtist(where: { Albums: { none: { Title: { contains: "the" }}}}) { Name }}"#),
            @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"{ findManyArtist(where: { Albums: { none: { Title: { contains: "album" }}}}) { Name }}"#),
            @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"}]}}"###
        );

        Ok(())
    }

    #[connector_test(only(Postgres))]
    async fn pg_rel1_some_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Title: { startsWith: "Album" }}}}, orderBy: { id: asc }) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: {  Title: { startsWith: "T" }}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}"###
        );

        Ok(())
    }

    #[connector_test(only(Postgres))]
    async fn pg_rel1_every_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { every: { Title: { contains: "Album" }}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Title: { not: { contains: "The" }}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        Ok(())
    }

    #[connector_test(only(Postgres))]
    async fn pg_rel1_none_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { none: { Title: { contains: "The" }}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { none: { Title: { contains: "Album" }}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn rel_filter_l2_some_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Tracks: { some: { Milliseconds: { lte: 9000 }}}}}}, orderBy: { Name: asc }) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Tracks: { some: { Bytes: { equals: 512 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn rel_filter_l2_all_filters(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // some|every
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Tracks: { every: { UnitPrice: { gt: 2.50 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Tracks: { every: { Milliseconds: { gt: 9000 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        // some|none
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Tracks: { none: { Milliseconds: { lte: 9000 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { some: { Tracks: { none: { UnitPrice: { lt: 2.0 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        // every|some
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { every: { Tracks: { some: { Bytes: { lt: 1000 }}}}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Tracks: { some: { Composer: {equals: "Composer3" }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"}]}}"###
        );

        // every|every
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Tracks: { every: { Bytes: { lte: 10000 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { every: { Tracks: { every: { TrackId: { in: [4,5,6,7] }}}}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        // every|none
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Tracks: { none: { UnitPrice: { lte: 1 }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { every: { Tracks: { none: { Composer: { equals: "Composer2" }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}"###
        );

        // none|some
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { none: { Tracks: { some: { UnitPrice: { lt: 1 }}}}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { none: { Tracks: { some: { Composer: { equals: "Composer2" }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}"###
        );

        // none|every
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { none: { Tracks: { every: { UnitPrice: { gte: 5 }}}}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyArtist(where: { Albums: { none: { Tracks: { every: { Name: { startsWith: "Track" }}}}}}) { Name }}"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"}]}}"###
        );

        // none|none
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { none: { Tracks: { none: { Bytes: { lt: 100 }}}}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyArtist(
              where: { Albums: { none: { Tracks: { none: { Bytes: { gte: 100 }}}}}}
              orderBy: { Name: asc }
            ) { Name }
          }"#),
          @r###"{"data":{"findManyArtist":[{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn rel_filter_l2_implicit_and_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { MediaType: { is: { Name: { equals: "MediaType1" }}}, Genre: { is: { Name: { equals: "Genre1" }}}}}}, orderBy: { id: asc }) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        Ok(())
    }

    #[connector_test()]
    async fn rel_filter_l2_implicit_and_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyAlbum(
              where: { Tracks: { every: { MediaType: { is: { Name: { equals: "MediaType1" }}}, Genre: { is: { Name: { equals: "Genre1" }}}}}}
              orderBy: { Title: asc }
            ) { Title }
          }"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"TheAlbumWithoutTracks"}]}}"###
        );

        Ok(())
    }

    #[connector_test()]
    async fn rel_filter_l2_explicit_and_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { AND:[ { MediaType: { is: { Name: { equals: "MediaType1" }}}}, { Genre: { is: { Name: { equals: "Genre1" }}}}]}}}, orderBy: { id: asc }) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { AND: [{ MediaType: { is: { Name: { equals: "MediaType2" }}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album3"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { AND:[] }}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        Ok(())
    }

    #[connector_test()]
    async fn rel_filter_l2_explicit_and_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
            findManyAlbum(
              where: { Tracks: { every: { AND: [{ MediaType: { is: { Name: { equals: "MediaType1" }}}}, { Genre: { is: { Name: { equals: "Genre1" }}}}]}}}
              orderBy: { Title: asc }
            ) { Title }
          }"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"TheAlbumWithoutTracks"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: { AND: [{ MediaType: { is: { Name: { equals: "MediaType2" }}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"TheAlbumWithoutTracks"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: { AND: [] }}}) { Title }}"#),

          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        Ok(())
    }

    #[connector_test()]
    async fn rel_filter_l2_explicit_or_all(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { OR:[{ MediaType: {is: { Name: { equals: "MediaType1" }}}}, { Genre: { is: { Name: { equals: "Genre2" }}}}]}}}, orderBy: { Title: asc }) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{
              findManyAlbum(
                where: { Tracks: { every: {OR:[{ MediaType: { is: { Name: { equals: "MediaType1"}}}},{Genre: { is: { Name: { equals: "Genre2"}}}}]}}}
                orderBy: { Title: asc }
              ) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"Album5"},{"Title":"TheAlbumWithoutTracks"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { OR:[{ MediaType: { is: { Name: { equals: "MediaType2"}}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album3"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: {OR:[{ MediaType: { is: { Name: { equals: "MediaType2"}}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"TheAlbumWithoutTracks"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { OR:[]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: {OR:[]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"TheAlbumWithoutTracks"}]}}"###
        );

        Ok(())
    }

    #[connector_test()]
    async fn rel_filter_l2_explicit_not_all(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { NOT: [{ MediaType: { is: { Name: { equals: "MediaType1"}}}},{Genre: { is: { Name: { equals: "Genre1"}}}}]}}}, orderBy: { Title: asc}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album3"},{"Title":"Album5"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: { NOT: [{ MediaType: { is: { Name: { equals: "MediaType1"}}}},{Genre: { is: { Name: { equals: "Genre1"}}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"TheAlbumWithoutTracks"},{"Title":"Album3"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { NOT: [{ MediaType: { is: { Name: { equals: "MediaType2"}}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: { NOT: [{ MediaType: { is: { Name: { equals: "MediaType2"}}}}]}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { some: { NOT: []}}}) { Title }}"#),
          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyAlbum(where: { Tracks: { every: { NOT: []}}}) { Title }}"#),

          @r###"{"data":{"findManyAlbum":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn rel_filter_l3(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! {r#"
            {
                findManyGenre(where: {
                  Tracks: {
                    some: { Album: { is: { Artist: { is: { ArtistId: { equals: 1 } } } } } }
                  }
                }) {
                  GenreId
                }
            }"#,
        }),
        @r###"{"data":{"findManyGenre":[{"GenreId":1}]}}"###);

        Ok(())
    }

    #[connector_test]
    async fn rel_scalar_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(&runner, indoc!{ r#"
              {
                findManyArtist(
                  where: {
                    Albums: {
                      some: {
                        Tracks: {
                          some: {
                            Genre: { is: { Name: { equals: "Genre1" } } }
                            TrackId: { equals: 1 }
                          }
                        }
                      }
                    }
                  }
                ) {
                  ArtistId
                }
              }
            "#}),
            @r###"{"data":{"findManyArtist":[{"ArtistId":1}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_none(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyGenre(where: { Tracks: { none: {} }}) { Name }}"#),
          @r###"{"data":{"findManyGenre":[{"Name":"GenreThatIsNotUsed"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn empty_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyGenre(where: { Tracks: { some: {} }}, orderBy: { Name: asc }) { Name }}"#),
          @r###"{"data":{"findManyGenre":[{"Name":"Genre1"},{"Name":"Genre2"},{"Name":"Genre3"}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneGenre(data: { Name: "Genre1", GenreId: 1}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneGenre(data: { Name: "Genre2", GenreId: 2}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneGenre(data: { Name: "Genre3", GenreId: 3}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneGenre(data: { Name: "GenreThatIsNotUsed", GenreId: 4}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneMediaType(data: { Name: "MediaType1", MediaTypeId: 1}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneMediaType(data: { Name: "MediaType2", MediaTypeId: 2}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneMediaType(data: { Name: "MediaType3", MediaTypeId: 3}) { Name }}"#)
            .await?
            .assert_success();

        runner
            .query(
                r#"mutation { createOneMediaType(data: { Name: "MediaTypeThatIsNotUsed", MediaTypeId: 4}) { Name }}"#,
            )
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
