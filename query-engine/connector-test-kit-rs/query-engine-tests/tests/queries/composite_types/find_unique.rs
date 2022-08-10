use query_engine_tests::*;

#[test_suite(only(MongoDb))]
mod find_unique {
    use query_engine_tests::assert_query;

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

    #[connector_test(schema(simple_uniq_idx_with_embedded))]
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

    fn uniq_idx_with_multiple_composite_fields() -> String {
        indoc! {r#"
        type Location {
            street  String
            zipCode String
            city    City
        }

        type City {
            name String
        }

        model A {
            #id(id, Int, @id)
            name String
            location Location

            @@unique([name, location.street, location.zipCode, location.city.name])
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(uniq_idx_with_multiple_composite_fields), only(MongoDb))]
    async fn multiple_fields_from_composite_type(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
            createManyA(data: [
                {id: 1 name: "foo" location: {set: {street: "a", zipCode: "a", city: { name: "paris" }}}},
                {id: 2 name: "foo" location: {set: {street: "b", zipCode: "b", city: { name: "paris" }}}},
                {id: 3 name: "bar" location: {set: {street: "c", zipCode: "c", city: { name: "paris" }}}},
            ]) { count }
        }"#}
        );

        assert_query!(
            runner,
            r#"query { findUniqueA(where: { 
                name_location_street_zipCode_city_name: {
                    name: "foo"
                    location: {
                        street: "a"
                        zipCode: "a",
                        city: {
                            name: "paris"
                        }
                    }
                } 
            }) { id }}"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn composite_uniq_idx_with_name() -> String {
        indoc! {r#"
        type Location {
            address Int
        }

        model A {
            #id(id, Int, @id)
            name String
            location Location

            @@unique([name, location.address], name: "name_address")
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(composite_uniq_idx_with_name), only(MongoDb))]
    async fn composite_unique_index_with_name(runner: Runner) -> TestResult<()> {
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
                name_address: {
                    name: "foo"
                    location: {
                        address: 1
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
}
