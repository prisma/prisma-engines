use query_engine_tests::*;

#[test_suite(schema(schema), only(Postgres))]
mod prisma_13097 {
    fn schema() -> String {
        r#"
enum AppCategories {
  calendar
  messaging
  payment
  other
}

model App {
  slug       String          @id @unique
  categories AppCategories[]
  createdAt  DateTime        @default(now())
  updatedAt  DateTime        @updatedAt
}

model Opp {
  slug       String          @id @unique
  categories Boolean[]
}
        "#
        .to_owned()
    }

    #[connector_test]
    async fn group_by_enum_array(runner: Runner) -> TestResult<()> {
        // Insert some data first
        run_query!(
            runner,
            r#"mutation { createManyApp(data: [{slug:"a",categories:[calendar,other]},{slug:"b",categories:[]},{slug:"c",categories:[calendar,other]},{slug:"d",categories:[messaging, payment]}]) { count } }"#
        );

        let result = run_query!(
            runner,
            r#"{groupByApp(by: [categories], orderBy: { categories: "desc" }) { _count { slug } categories }}"#
        );
        assert_eq!(
            result,
            "{\"data\":{\"groupByApp\":[{\"_count\":{\"slug\":1},\"categories\":[\"messaging\",\"payment\"]},{\"_count\":{\"slug\":2},\"categories\":[\"calendar\",\"other\"]},{\"_count\":{\"slug\":1},\"categories\":[]}]}}"
        );

        let result = run_query!(
            runner,
            r#"{groupByApp(by: [categories], orderBy: { categories: "asc" }) { _count { slug categories } }}"#
        );
        assert_eq!(
            result,
            "{\"data\":{\"groupByApp\":[{\"_count\":{\"slug\":1,\"categories\":1}},{\"_count\":{\"slug\":2,\"categories\":2}},{\"_count\":{\"slug\":1,\"categories\":1}}]}}"
        );

        Ok(())
    }

    #[connector_test]
    async fn group_by_boolean_array(runner: Runner) -> TestResult<()> {
        // Insert some data first
        run_query!(
            runner,
            r#"mutation { createManyOpp(data: [{slug:"a",categories:[true,false]},{slug:"b",categories:[]},{slug:"c",categories:[false,true]},{slug:"d",categories:[true,false]}]) { count } }"#
        );

        let result = run_query!(
            runner,
            r#"{groupByOpp(by: [categories], orderBy: { categories: "desc" }) { _count { slug } categories }}"#
        );
        assert_eq!(
            result,
            "{\"data\":{\"groupByOpp\":[{\"_count\":{\"slug\":2},\"categories\":[true,false]},{\"_count\":{\"slug\":1},\"categories\":[false,true]},{\"_count\":{\"slug\":1},\"categories\":[]}]}}"
        );

        Ok(())
    }
}
