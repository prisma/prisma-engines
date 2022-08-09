use query_engine_tests::*;

#[test_suite(only(MongoDb))]
mod find_unique {
    use query_engine_tests::assert_query;
    use query_tests_setup::TestResult;

    fn simple_uniq_idx_with_embedded() -> String {
        indoc! {r#"
        type Location {
            address String
        }

        model A {
            #id(id, Int, @id)
            name String
            location Location

            @@unique([location.address])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(simple_uniq_idx_with_embedded), only(MongoDb))]
    async fn simple_embedded_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
            createManyA(data: [
                {id: 1 name: "foo" location: {set: {address: "a"}}},
                {id: 2 name: "foo" location: {set: {address: "b"}}},
                {id: 3 name: "foo" location: {set: {address: "c"}}},
            ]) { count }
        }"#}
        );

        assert_query!(
            runner,
            r#"query { findUniqueA(where: { 
                location_address: {
                    location: {
                        address: "a"
                    }
                }
            }) { id }}"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn two_composite_uniq_idx() -> String {
        indoc! {r#"
        type Location {
            address String
        }

        type Person {
            name String
            age Int
        }

        model A {
            #id(id, Int, @id)
            person Person
            location Location

            @@unique([location.address, person.name])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(two_composite_uniq_idx), only(MongoDb))]
    async fn two_composite_unique_idx(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
                createManyA(data: [
                  {id: 1 person: {name: "foo", age: 1}, location: {address: "a"}},
                  {id: 2 person: {name: "bar", age: 2}, location: {address: "a"}},
                  {id: 3 person: {name: "foo", age: 3}, location: {address: "b"}},
                ]) { count }
              }"#}
        );

        assert_query!(
            runner,
            r#"query {
                findUniqueA(where: {
                  location_address_person_name: {
                    location: {
                      address: "a"
                    },
                    person: {
                        name: "foo",
                    }
                  }
                }) { id }
              }"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn composite_uniq_idx_with_embedded() -> String {
        indoc! {r#"
        type Location {
            address String
        }

        model A {
            #id(id, Int, @id)
            name String
            location Location

            @@unique([name, location.address])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(composite_uniq_idx_with_embedded), only(MongoDb))]
    async fn composite_embedded_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
            createManyA(data: [
                {id: 1 name: "foo" location: {set: {address: "a"}}},
                {id: 2 name: "foo" location: {set: {address: "b"}}},
                {id: 3 name: "bar" location: {set: {address: "c"}}},
            ]) { count }
        }"#}
        );

        assert_query!(
            runner,
            r#"query { findUniqueA(where: { 
                name_location_address: {
                    name: "foo"
                    location: {
                        address: "a"
                    }
                } 
            }) { id }}"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn composite_uniq_idx_with_embedded_list() -> String {
        indoc! {r#"
        type Location {
            address String
        }

        model A {
            #id(id, Int, @id)
            name String
            locations Location[]

            @@unique([locations.address])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(composite_uniq_idx_with_embedded_list), only(MongoDb))]
    async fn composite_embedded_list_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
                createManyA(data: [
                  {id: 1 name: "foo" locations: { set: [{address: "a"}, {address: "b"}] }},
                  {id: 2 name: "bar" locations: { set: [{address: "c"}] }}
                ]) { count }
              }"#}
        );

        assert_query!(
            runner,
            r#"query {
                findUniqueA(where: {
                  locations_address: {
                    locations: {
                      address: "a"
                    }
                  }
                }) { id }
              }"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn two_fields_same_composite() -> String {
        indoc! {r#"
        type B {
            foo String
            bar String
        }

        model A {
            #id(id, Int, @id)
            b B

            @@unique([b.foo, b.bar])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(two_fields_same_composite), only(MongoDb))]
    async fn two_fields_from_same_composite_uniq_idx(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
                createManyA(data: [
                  {id: 1 b: { foo: "a" bar: "b" } },
                  {id: 2 b: { foo: "a" bar: "c" } }
                ]) { count }
              }"#}
        );

        assert_query!(
            runner,
            r#"query {
                findUniqueA(where: {
                  b_foo_b_bar: {
                    b: {
                      foo: "a"
                      bar: "b"
                    }
                  }
                }) { id }
              }"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }
}
