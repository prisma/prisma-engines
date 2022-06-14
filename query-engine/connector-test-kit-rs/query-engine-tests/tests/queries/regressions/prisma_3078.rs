use query_engine_tests::*;

// validates fix for
//https://github.com/prisma/prisma/issues/3078
//https://github.com/prisma/prisma-client-js/issues/550

// The relationfilter logic for Selfrelations was sensitive to the side from which the filter traversed as well as the
// naming of the relationfields since this fed into the RelationSide logic. This tests traversal from both sides as well
// as switching the lexicographic order of the relation fields.

// /!\ rel_filter_1_1_a and rel_filter_1_1_z must always expect the same results
// /!\ rel_filter_1_m_a and rel_filter_1_m_z must always expect the same results
// /!\ rel_filter_n_m_a and rel_filter_n_m_z must always expect the same results

#[test_suite]
mod prisma_3078 {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn rel_filter_1_1_a() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name       String?
              field_b    User?    @relation("UserfriendOf")
              field_a    User?    @relation("UserfriendOf", fields: [field_aId], references: [id], onDelete: NoAction, onUpdate: NoAction)
              field_aId  Int? @unique
            }"#
        };

        schema.to_owned()
    }

    fn rel_filter_1_1_z() -> String {
        let schema = indoc! {
            r#"model User {
            #id(id, Int, @id)
            name       String?
            field_b    User?    @relation("UserfriendOf")
            field_z    User?    @relation("UserfriendOf", fields: [field_zId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            field_zId  Int? @unique
          }"#
        };

        schema.to_owned()
    }

    fn rel_filter_1_m_a() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name      String?
              field_b   User[]  @relation("UserfriendOf")
              field_a   User?   @relation("UserfriendOf", fields: [field_aId], references: [id], onDelete: NoAction, onUpdate: NoAction)
              field_aId Int?
            }"#
        };

        schema.to_owned()
    }

    fn rel_filter_1_m_z() -> String {
        let schema = indoc! {
            r#"model User {
            #id(id, Int, @id)
            name      String?
            field_b   User[]  @relation("UserfriendOf")
            field_z   User?   @relation("UserfriendOf", fields: [field_zId], references: [id], onDelete: NoAction, onUpdate: NoAction)
            field_zId Int?
          }"#
        };

        schema.to_owned()
    }

    fn rel_filter_n_m_a() -> String {
        // field_b    User[]  @relation("UserfriendOf")
        // field_a    User[]  @relation("UserfriendOf")
        let schema = indoc! {
            r#"model User {
            #id(id, Int, @id)
            name       String?
            #m2m(field_b, User[], id, Int, UserfriendOf)
            #m2m(field_a, User[], id, Int, UserfriendOf)
            field_aId  Int?
          }"#
        };

        schema.to_owned()
    }

    fn rel_filter_n_m_z() -> String {
        // field_b    User[]  @relation("UserfriendOf")
        // field_z    User[]  @relation("UserfriendOf")
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              name       String?
              #m2m(field_b, User[], id, Int, UserfriendOf)
              #m2m(field_z, User[], id, Int, UserfriendOf)
              field_zId  Int?
            }"#
        };

        schema.to_owned()
    }

    // "A relation filter on a 1:1 self relation " should "work" (with field_a)
    #[connector_test(schema(rel_filter_1_1_a), exclude(SqlServer))]
    async fn relation_filter_1_1_a(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 1, name: "A", field_a:{ create:{ id: 10, name: "AA"}}}){
            id
            field_b { id }
            field_a { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":1,"field_b":null,"field_a":{"id":10}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 2, name: "B", field_a:{ create:{ id: 20, name: "BB"}}}){
            id
            field_b { id }
            field_a { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":2,"field_b":null,"field_a":{"id":20}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_a:{ is:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_a { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"field_b":null,"field_a":{"id":20,"name":"BB"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_b:{ is:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_a { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":20,"field_b":{"id":2,"name":"B"},"field_a":null}]}}"###
        );

        Ok(())
    }

    // "A relation filter on a 1:1 self relation " should "work" (with field_z)
    #[connector_test(schema(rel_filter_1_1_z), exclude(SqlServer))]
    async fn relation_filter_1_1_z(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 1, name: "A", field_z:{ create:{ id: 10, name: "AA"}}}){
                id
                field_b { id }
                field_z { id }
              }
            }"#),
          @r###"{"data":{"createOneUser":{"id":1,"field_b":null,"field_z":{"id":10}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 2, name: "B", field_z:{ create:{ id: 20, name: "BB"}}}){
                id
                field_b { id }
                field_z { id }
              }
            }"#),
          @r###"{"data":{"createOneUser":{"id":2,"field_b":null,"field_z":{"id":20}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_z:{ is:{ name: {contains: "B"}}}}){
                id
                field_b { id, name}
                field_z { id, name }
              }
            }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"field_b":null,"field_z":{"id":20,"name":"BB"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_b:{ is:{ name: {contains: "B"}}}}){
                id
                field_b { id, name}
                field_z { id, name }
              }
            }"#),
          @r###"{"data":{"findManyUser":[{"id":20,"field_b":{"id":2,"name":"B"},"field_z":null}]}}"###
        );

        Ok(())
    }

    //"A relation filter on a 1:M self relation " should "work"
    #[connector_test(schema(rel_filter_1_m_a))]
    async fn relation_filter_1_m_a(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 1, name: "A" field_a:{ create:{ id: 10, name: "AA"}}}){
            id
            field_b { id }
            field_a { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":1,"field_b":[],"field_a":{"id":10}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 2, name: "B" field_a:{ create:{ id: 20, name: "BB"}}}){
            id
            field_b { id }
            field_a { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":2,"field_b":[],"field_a":{"id":20}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_a:{ is:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_a { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"field_b":[],"field_a":{"id":20,"name":"BB"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_b:{ some:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_a { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":20,"field_b":[{"id":2,"name":"B"}],"field_a":null}]}}"###
        );

        Ok(())
    }

    //"A relation filter on a 1:M self relation " should "work"
    #[connector_test(schema(rel_filter_1_m_z))]
    async fn relation_filter_1_m_z(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 1, name: "A" field_z:{ create:{ id: 10, name: "AA"}}}){
            id
            field_b { id }
            field_z { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":1,"field_b":[],"field_z":{"id":10}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 2, name: "B" field_z:{ create:{ id: 20, name: "BB"}}}){
            id
            field_b { id }
            field_z { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":2,"field_b":[],"field_z":{"id":20}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_z:{ is:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_z { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"field_b":[],"field_z":{"id":20,"name":"BB"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_b:{ some:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_z { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":20,"field_b":[{"id":2,"name":"B"}],"field_z":null}]}}"###
        );

        Ok(())
    }

    // "A relation filter on a N:M self relation " should "work"
    #[connector_test(schema(rel_filter_n_m_a))]
    async fn relation_filter_n_m_a(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 1, name: "A" field_a:{ create:{ id: 10, name: "AA"}}}){
            id
            field_b { id }
            field_a { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":1,"field_b":[],"field_a":[{"id":10}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 2, name: "B" field_a:{ create:{ id: 20, name: "BB"}}}){
            id
            field_b { id }
            field_a { id }
          }
        }"#),
          @r###"{"data":{"createOneUser":{"id":2,"field_b":[],"field_a":[{"id":20}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_a:{ some:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_a { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"field_b":[],"field_a":[{"id":20,"name":"BB"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_b:{ some:{ name: {contains: "B"}}}}){
            id
            field_b { id, name}
            field_a { id, name }
          }
        }"#),
          @r###"{"data":{"findManyUser":[{"id":20,"field_b":[{"id":2,"name":"B"}],"field_a":[]}]}}"###
        );

        Ok(())
    }

    // "A relation filter on a N:M self relation " should "work"
    #[connector_test(schema(rel_filter_n_m_z))]
    async fn relation_filter_n_m_z(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 1, name: "A" field_z:{ create:{ id: 10, name: "AA"}}}){
                id
                field_b { id }
                field_z { id }
              }
            }"#),
          @r###"{"data":{"createOneUser":{"id":1,"field_b":[],"field_z":[{"id":10}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation{createOneUser(data: { id: 2, name: "B" field_z:{ create:{ id: 20, name: "BB"}}}){
                id
                field_b { id }
                field_z { id }
              }
            }"#),
          @r###"{"data":{"createOneUser":{"id":2,"field_b":[],"field_z":[{"id":20}]}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_z:{ some:{ name: {contains: "B"}}}}){
                id
                field_b { id, name}
                field_z { id, name }
              }
            }"#),
          @r###"{"data":{"findManyUser":[{"id":2,"field_b":[],"field_z":[{"id":20,"name":"BB"}]}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{findManyUser(where: { field_b:{ some:{ name: {contains: "B"}}}}){
                id
                field_b { id, name}
                field_z { id, name }
              }
            }"#),
          @r###"{"data":{"findManyUser":[{"id":20,"field_b":[{"id":2,"name":"B"}],"field_z":[]}]}}"###
        );

        Ok(())
    }
}
