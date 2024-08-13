use query_engine_tests::*;

#[test_suite(schema(schema))]
mod self_relation_filters {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Human {
                #id(id, String, @id)
                name       String
                wife_id    String? @unique
                mother_id  String?
                father_id  String?
                singer_id  String?
                title_id   String? @unique

                husband       Human? @relation(name: "Marriage")
                wife          Human? @relation(name: "Marriage",  fields: [wife_id],   references: [id], onDelete: NoAction, onUpdate: NoAction)
                mother        Human? @relation(name: "Cuckoo",    fields: [mother_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
                father        Human? @relation(name: "Offspring", fields: [father_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
                singer        Human? @relation(name: "Team",      fields: [singer_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
                title         Song?  @relation(                   fields: [title_id],  references: [id], onDelete: NoAction, onUpdate: NoAction)

                daughters     Human[] @relation(name: "Offspring")
                stepdaughters Human[] @relation(name: "Cuckoo")
                bandmembers   Human[] @relation(name: "Team")

                #m2m(fans, Human[], id, String, Admirers)
                #m2m(rockstars, Human[], id, String, Admirers)
            }

            model Song{
                #id(id, String, @id)
                title   String
                creator Human?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    // Filter Queries along self relations should succeed with one level.
    // On D1, this test fails with a panic:
    // ```
    // {"errors":[{"error":"RecordNotFound(\"Expected 1 records to be connected after connect operation on one-to-many relation 'Cuckoo', found 4.\")","user_facing_error":{"is_panic":false,"message":"The required connected records were not found. Expected 1 records to be connected after connect operation on one-to-many relation 'Cuckoo', found 4.","meta":{"details":"Expected 1 records to be connected after connect operation on one-to-many relation 'Cuckoo', found 4."},"error_code":"P2018"}}]}
    // ```
    async fn l1_query(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { name: { equals: "kurt" }}}}, orderBy: { title: desc }) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"My Girl"},{"title":"Gasag"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along self relations should succeed with two levels.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn l2_query(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(
                where: {
                  creator: { is: { daughters: { some: { name: { equals: "frances" } } } } }
                }
              ) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"My Girl"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along OneToOne self relations should succeed with two levels.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn l2_one2one(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(
                where: { creator: { is: { wife: { is: { name: { equals: "yoko" } } } } } }
              ) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"Imagine"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along OneToOne self relations should succeed with null filter.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn one2one_null(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { wife: { is: null }}}}) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"Bicycle"},{"title":"Gasag"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along OneToOne self relations should succeed with {} filter.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn one2one_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { wife: { is: {} } } } }) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"My Girl"},{"title":"Imagine"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along OneToMany self relations should fail with null filter.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn one2one_null_fail(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        assert_error!(
            runner,
            indoc! { r#"
              query {
                findManySong(where: { creator: { is: { daughters: { none: null } } } }) {
                  title
                }
              }
            "#,
            },
            2009,
            "`where.creator.is.daughters.none`: A value is required but not set"
        );

        Ok(())
    }

    // Filter Queries along OneToMany self relations should succeed with empty filter (`{}`).
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn one2many_empty(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { daughters: { some: {} } } } }) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"My Girl"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should succeed with valid filter `some`.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(
                where: { creator: { is: { fans: { some: { name: { equals: "groupie1" }}}}}}
                orderBy: { id: asc }
              ) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"My Girl"},{"title":"Imagine"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should succeed with valid filter `none`.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_none(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { fans: { none: { name: { equals: "groupie1" }}}}}}) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"Bicycle"},{"title":"Gasag"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should succeed with valid filter `every`.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { fans: { every: { name: { equals: "groupie1" }}}}}}) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"Imagine"},{"title":"Bicycle"},{"title":"Gasag"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should give an error with null.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_null_error(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        assert_error!(
            runner,
            indoc! { r#"
              query {
                findManySong(
                  where: { creator: { is: { fans: { every: { fans: { some: null } } } } } }
                ) {
                  title
                }
              }"#,
            },
            2009,
            "A value is required but not set"
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should succeed with {} filter `some`.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_empty_some(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManySong(where: { creator: { is: { fans: { some: {} } } } }) {
                title
              }
            }
          "# }),
          @r###"{"data":{"findManySong":[{"title":"My Girl"},{"title":"Imagine"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should succeed with {} filter `none`.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_empty_none(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // Note: Result ordering changed for the ported tests, but the result is correct.
        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyHuman(where: { fans: { none: {} } }, orderBy: { id: asc }) {
                name
              }
            }
          "# }),
          @r###"{"data":{"findManyHuman":[{"name":"paul"},{"name":"freddy"},{"name":"kurt"},{"name":"dave"},{"name":"groupie1"},{"name":"groupie2"},{"name":"frances"},{"name":"courtney"},{"name":"yoko"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToMany self relations should succeed with {} filter `every`.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2many_empty_every(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        // Note: Result ordering changed for the ported tests, but the result is correct.
        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyHuman(where: { fans: { every: {} } }, orderBy: { id: asc }) {
                name
              }
            }
          "# }),
          @r###"{"data":{"findManyHuman":[{"name":"paul"},{"name":"freddy"},{"name":"kurt"},{"name":"dave"},{"name":"groupie1"},{"name":"groupie2"},{"name":"frances"},{"name":"courtney"},{"name":"kurt"},{"name":"yoko"},{"name":"john"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToOne self relations should succeed valid filter.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2one(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyHuman(where: { singer: { is: { name: { equals: "kurt" } } } }) {
                name
              }
            }
          "# }),
          @r###"{"data":{"findManyHuman":[{"name":"dave"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToOne self relations should succeed with {} filter.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2one_empty_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyHuman(where: { singer: { is: {} } }, orderBy: { id: asc }) {
                name
              }
            }
          "# }),
          @r###"{"data":{"findManyHuman":[{"name":"paul"},{"name":"dave"}]}}"###
        );

        Ok(())
    }

    // Filter Queries along ManyToOne self relations should succeed with null filter.
    #[connector_test(exclude(SqlServer, Sqlite("cfd1")))]
    async fn many2one_null_filter(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyHuman(where: { singer: { is: null } }, orderBy: { id: asc }) {
                name
              }
            }
          "# }),
          @r###"{"data":{"findManyHuman":[{"name":"freddy"},{"name":"kurt"},{"name":"groupie1"},{"name":"groupie2"},{"name":"frances"},{"name":"courtney"},{"name":"kurt"},{"name":"yoko"},{"name":"john"}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneHuman(data: { id: "1", name: "paul" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "2", name: "dave" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "3", name: "groupie1" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "4", name: "groupie2" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "5", name: "frances" }) { id }}"#)
            .await?
            .assert_success();

        runner.query(r#"mutation { createOneHuman(data: { id: "6", name: "courtney",stepdaughters: { connect: [{ id: "5" }]}}) { id }}"#).await?.assert_success();

        runner
            .query(indoc! { r#"
                mutation {
                    createOneHuman(
                        data: {
                            id: "7",
                            name: "kurt"
                            wife: { connect: { id: "6" } }
                            daughters: { connect: [{ id: "5" }] }
                            fans: { connect: [{ id: "3" }, { id: "4" }] }
                            bandmembers: { connect: [{ id: "2" }] }
                        }
                    ) { id }
                }
            "#})
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { id: "s1", title: "My Girl", creator: { connect: { id: "7" }}}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation{ createOneHuman(data: { id: "8", name: "yoko" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation {
                    createOneHuman(
                        data: {
                            id: "9",
                            name: "john"
                            wife: { connect: { id: "8" } }
                            fans: { connect: [{ id: "3" }] }
                            bandmembers: { connect: [{ id: "1" }] }
                        }
                    ) { id }
                }
            "#})
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { id: "s2", title: "Imagine", creator: { connect: { id: "9" }}}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "10", name: "freddy" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { id: "s3", title: "Bicycle", creator: { connect: { id: "10" }}}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "11", name: "kurt" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { id: "s4", title: "Gasag", creator: { connect: { id: "11" }}}) { id }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
