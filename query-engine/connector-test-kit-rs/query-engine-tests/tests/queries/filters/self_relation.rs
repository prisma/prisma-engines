use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod self_relation_filters {
    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Human {
                #id(id, String, @id)
                name       String
                wife_id    String?
                mother_id  String?
                father_id  String?
                singer_id  String?
                title_id   String?

                husband       Human? @relation(name: "Marriage")
                wife          Human? @relation(name: "Marriage",  fields: [wife_id],   references: [id])
                mother        Human? @relation(name: "Cuckoo",    fields: [mother_id], references: [id])
                father        Human? @relation(name: "Offspring", fields: [father_id], references: [id])
                singer        Human? @relation(name: "Team",      fields: [singer_id], references: [id])
                title         Song?  @relation(                   fields: [title_id],  references: [id])

                daughters     Human[] @relation(name: "Offspring")
                stepdaughters Human[] @relation(name: "Cuckoo")
                fans          Human[] @relation(name: "Admirers")
                rockstars     Human[] @relation(name: "Admirers")
                bandmembers   Human[] @relation(name: "Team")
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

    // Filter Queries along self relations should succeed with one level.
    #[connector_test]
    async fn l1_query(runner: &Runner) -> TestResult<()> {
        test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, indoc! { r#"
          query {
            findManySong(
              where: {
                creator: {
                  is: {
                    name: { equals: "kurt" }
                  }
                }
              }
            ) {
              title
            }
          "# }),
          @r###""###
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
            .query(r#"mutation { createOneSong(data: { title: "My Girl", creator: { connect: { id: "7" }}}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation{createOneHuman(data: { id: 8, name: "yoko" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(indoc! { r#"
                mutation {
                    createOneHuman(
                        data: {
                            id: "9",
                            name: "john"
                            wife: { connect: { id: "9" } }
                            fans: { connect: [{ id: "3" }] }
                            bandmembers: { connect: [{ id: "1" }] }
                        }
                    ) { id }
                }
            "#})
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { title: "Imagine", creator: { connect: { id: "9" }}}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "10", name: "freddy" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { title: "Bicycle", creator: { connect: { id: "10" }}}) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneHuman(data: { id: "11", name: "kurt" }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneSong(data: { title: "Gasag", creator: { connect: { id: "11" }}}) { id }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}
