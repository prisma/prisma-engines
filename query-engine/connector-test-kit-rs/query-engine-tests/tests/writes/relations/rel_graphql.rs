use query_engine_tests::*;

#[test_suite(schema(schema))]
mod rel_graphql {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Owner{
              #id(id, String, @id, @default(cuid()))
              ownerName String? @unique
              cat       Cat?
           }

           model Cat{
              #id(id, String, @id, @default(cuid()))
              catName String? @unique
              ownerId String?

              owner   Owner?  @relation(fields: [ownerId], references: [id])
           }"#
        };

        schema.to_owned()
    }

    // "One2One relations" should "only allow one item per side"
    #[connector_test(exclude(SqlServer))]
    async fn one2one_rel_allow_one_item_per_side(runner: Runner) -> TestResult<()> {
        create_row(&runner, "Cat", "garfield").await?;
        create_row(&runner, "Cat", "azrael").await?;
        create_row(&runner, "Owner", "jon").await?;
        create_row(&runner, "Owner", "gargamel").await?;

        //set initial owner
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneCat(
            where: {catName: "garfield"},
            data: {owner: {connect: {ownerName: "jon"}}}) {
              catName
              owner {
                ownerName
              }
            }
          }"#),
          @r###"{"data":{"updateOneCat":{"catName":"garfield","owner":{"ownerName":"jon"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findUniqueOwner(where:{ownerName:"jon"}){ownerName, cat{catName}}}"#),
          @r###"{"data":{"findUniqueOwner":{"ownerName":"jon","cat":{"catName":"garfield"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findUniqueOwner(where:{ownerName:"gargamel"}){ownerName, cat{catName}}}"#),
          @r###"{"data":{"findUniqueOwner":{"ownerName":"gargamel","cat":null}}}"###
        );

        //change owner
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {updateOneCat(where: {catName: "garfield"},
          data: {owner: {connect: {ownerName: "gargamel"}}}) {
              catName
              owner {
                ownerName
              }
            }
          }"#),
          @r###"{"data":{"updateOneCat":{"catName":"garfield","owner":{"ownerName":"gargamel"}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findUniqueOwner(where:{ownerName:"jon"}){ownerName, cat{catName}}}"#),
          @r###"{"data":{"findUniqueOwner":{"ownerName":"jon","cat":null}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query{findUniqueOwner(where:{ownerName:"gargamel"}){ownerName, cat{catName}}}"#),
          @r###"{"data":{"findUniqueOwner":{"ownerName":"gargamel","cat":{"catName":"garfield"}}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, model_name: &str, name: &str) -> TestResult<()> {
        match model_name {
            "Cat" => runner
                .query(format!(
                    "mutation {{ createOneCat(data: {{ catName: \"{}\" }}) {{ id }} }}",
                    name
                ))
                .await?
                .assert_success(),
            "Owner" => runner
                .query(format!(
                    "mutation {{ createOneOwner(data: {{ ownerName: \"{}\" }}) {{ id }} }}",
                    name
                ))
                .await?
                .assert_success(),
            _ => unreachable!(),
        }
        Ok(())
    }
}
