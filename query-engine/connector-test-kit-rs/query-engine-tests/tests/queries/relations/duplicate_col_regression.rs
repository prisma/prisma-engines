use query_engine_tests::*;

#[test_suite(schema(schema))]
mod dup_col_regr {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Transcriber {
              #id(id, String, @id)
              competencies     TranscriberCompetency[]
            }

            model TranscriberCompetency {
              #id(id, String, @id)
              transcriber   Transcriber @relation(fields: [transcriberId], references: [id])
              transcriberId String
              competency    Competency  @relation(fields: [competencyId], references: [id])
              competencyId  String
              @@unique([transcriberId, competencyId])
            }

            model Competency {
              #id(id, String, @id)
              transcriberCompetencies     TranscriberCompetency[]
            }"#
        };

        schema.to_owned()
    }

    // "Querying a scalarfield that would already be included since it backs a relationfield" should "only request the underlying column once"
    #[connector_test]
    async fn test_1(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneTranscriber(data: { id: "one", competencies: { create: { id: "one_trans", competency: {create:{ id: "one_comp"}}} } }){
              id
              competencies{
              id
              transcriberId
              competency
               {id}
              }

            }
          }"#),
          @r###"{"data":{"createOneTranscriber":{"id":"one","competencies":[{"id":"one_trans","transcriberId":"one","competency":{"id":"one_comp"}}]}}}"###
        );

        Ok(())
    }
}
