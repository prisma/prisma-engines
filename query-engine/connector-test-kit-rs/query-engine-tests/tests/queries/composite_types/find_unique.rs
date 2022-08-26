use query_engine_tests::*;

#[test_suite(only(MongoDb))]
mod find_unique {
    use query_engine_tests::assert_query;

    fn simple_partial_schema() -> String {
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

    #[connector_test(schema(simple_partial_schema))]
    async fn simple_partial(runner: Runner) -> TestResult<()> {
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

    fn compound_partial_schema() -> String {
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

    #[connector_test(schema(compound_partial_schema), only(MongoDb))]
    async fn compound_partial(runner: Runner) -> TestResult<()> {
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

    fn simple_list_partial_schema() -> String {
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

    #[connector_test(schema(simple_list_partial_schema), only(MongoDb))]
    async fn simple_list_partial(runner: Runner) -> TestResult<()> {
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

    fn compound_mix_schema() -> String {
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

    #[connector_test(schema(compound_mix_schema), only(MongoDb))]
    async fn compound_and_composite_partial_mix(runner: Runner) -> TestResult<()> {
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

    fn compound_with_name_schema() -> String {
        indoc! {r#"
        type Location {
            address String
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

    #[connector_test(schema(compound_with_name_schema), only(MongoDb))]
    async fn compound_with_name(runner: Runner) -> TestResult<()> {
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
                        address: "a"
                    }
                } 
            }) { id }}"#,
            r#"{"data":{"findUniqueA":{"id":1}}}"#
        );

        Ok(())
    }

    fn compound_partial_composites_schema() -> String {
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

    #[connector_test(schema(compound_partial_composites_schema), only(MongoDb))]
    async fn compound_partial_composites(runner: Runner) -> TestResult<()> {
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

    fn compound_full_composite_idx_schema() -> String {
        indoc! {r#"
        model Test {
            #id(id, Int, @id)
            person Person
            location Location

            @@unique([location, person])
        }

        type Location {
            address Address
            gps String
        }

        type Person {
            name String
            age Int
        }

        type Address {
            address String
        }
        "#}
        .to_string()
    }

    #[connector_test(schema(compound_full_composite_idx_schema), only(MongoDb))]
    async fn compound_full_composite_idx(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            indoc! {r#"mutation {
                createManyTest(data: [
                  {id: 1, person: {name: "foo", age: 1}, location: {address: { address: "a" }, gps: "north"}},
                  {id: 2, person: {name: "bar", age: 2}, location: {address: { address: "a" }, gps: "east"}},
                  {id: 3, person: {name: "foo", age: 3}, location: {address: { address: "b" }, gps: "north"}},
                ]) { count }
              }"#}
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"query {
            findUniqueTest(where: {
              location_person: {
                location: {
                  address: { address: "a" }
                  gps: "east"
                },
                person: {
                  name: "bar",
                  age: 2,
                }
              }
            }) { id }
          }"#),
          @r###"{"data":{"findUniqueTest":{"id":2}}}"###
        );

        Ok(())
    }

    fn full_composite_uniq_idx_schema() -> String {
        let schema = indoc! {
            r#"model Test {
              #id(id, Int, @id)
              int       Int
              comp      Composite @unique
              comp_list Composite[] @unique

              @@unique([comp, comp_list])
              @@unique([int, comp])
              @@unique([int, comp_list])
              @@unique([int, comp, comp_list])
          }
          
          type Composite {
            int         Int
            int_list    Int[]
            nested      NestedComposite
            nested_list NestedComposite[]
          }
          
          type NestedComposite {
            int      Int
            int_list Int[]
          }            
          "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(full_composite_uniq_idx_schema))]
    async fn full_compound_composite(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
            createManyTest(
              data: [
                {
                  id: 1
                  int: 1
                  comp: {
                    int: 1
                    int_list: [1, 2]
                    nested: { int: 1, int_list: [1, 2] }
                    nested_list: [
                      { int: 1, int_list: [1, 2] }
                      { int: 2, int_list: [1, 2] }
                    ]
                  }
                  comp_list: [
                    {
                      int: 1
                      int_list: [1, 2]
                      nested: { int: 1, int_list: [1, 2] }
                      nested_list: [
                        { int: 1, int_list: [1, 2] }
                        { int: 2, int_list: [1, 2] }
                      ]
                    }
                    {
                      int: 2
                      int_list: [1, 2]
                      nested: { int: 1, int_list: [1, 2] }
                      nested_list: [
                        { int: 1, int_list: [1, 2] }
                        { int: 2, int_list: [1, 2] }
                      ]
                    }
                  ]
                }
                {
                  id: 2
                  int: 1
                  comp: {
                    int: 2
                    int_list: [1, 2]
                    nested: { int: 1, int_list: [1, 2] }
                    nested_list: [
                      { int: 1, int_list: [1, 2] }
                      { int: 2, int_list: [1, 2] }
                    ]
                  }
                  comp_list: [
                    {
                      int: 3
                      int_list: [1, 2]
                      nested: { int: 1, int_list: [1, 2] }
                      nested_list: [
                        { int: 1, int_list: [1, 2] }
                        { int: 2, int_list: [1, 2] }
                      ]
                    }
                    {
                      int: 4
                      int_list: [1, 2]
                      nested: { int: 1, int_list: [1, 2] }
                      nested_list: [
                        { int: 1, int_list: [1, 2] }
                        { int: 2, int_list: [1, 2] }
                      ]
                    }
                  ]
                }
              ]
            ) {
              count
            }
          }
          "#
        );

        // Scalar unique index
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTest(where: { id: 1 }) { id } } "#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        // Composite unique index
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findUniqueTest(
            where: {
              comp: {
                int: 1
                int_list: [1, 2]
                nested: { int: 1, int_list: [1, 2] }
                nested_list: [
                  { int: 1, int_list: [1, 2] }
                  { int: 2, int_list: [1, 2] }
                ]
              }
            }
          ) {
            id
          }
        }          
        "#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        // Composite list unique index
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findUniqueTest(
            where: {
              comp_list: {
                int: 1
                int_list: [1, 2]
                nested: { int: 1, int_list: [1, 2] }
                nested_list: [
                  { int: 1, int_list: [1, 2] }
                  { int: 2, int_list: [1, 2] }
                ]
              }
            }
          ) {
            id
          }
        }
        "#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        // Compound unique index: scalar + composite
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findUniqueTest(
            where: {
              int_comp: {
                int: 1
                comp: {
                  int: 1
                  int_list: [1, 2]
                  nested: { int: 1, int_list: [1, 2] }
                  nested_list: [
                    { int: 1, int_list: [1, 2] }
                    { int: 2, int_list: [1, 2] }
                  ]
                }
              }
            }
          ) {
            id
          }
        }"#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        // Compound unique index: scalar + composite list
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findUniqueTest(
            where: {
              int_comp_list: {
                int: 1
                comp_list: {
                  int: 1
                  int_list: [1, 2]
                  nested: { int: 1, int_list: [1, 2] }
                  nested_list: [
                    { int: 1, int_list: [1, 2] }
                    { int: 2, int_list: [1, 2] }
                  ]
                }
              }
            }
          ) {
            id
          }
        }
        "#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        // Compound unique index: composite + composite list
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findUniqueTest(
            where: {
              comp_comp_list: {
                comp: {
                  int: 1
                  int_list: [1, 2]
                  nested: { int: 1, int_list: [1, 2] }
                  nested_list: [
                    { int: 1, int_list: [1, 2] }
                    { int: 2, int_list: [1, 2] }
                  ]
                }
                comp_list: {
                  int: 1
                  int_list: [1, 2]
                  nested: { int: 1, int_list: [1, 2] }
                  nested_list: [
                    { int: 1, int_list: [1, 2] }
                    { int: 2, int_list: [1, 2] }
                  ]
                }
              }
            }
          ) {
            id
          }
        }
        "#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        // Compound unique index: scalar + composite + composite list
        insta::assert_snapshot!(
          run_query!(&runner, r#"{
          findUniqueTest(
            where: {
              int_comp_comp_list: {
                int: 1
                comp: {
                  int: 1
                  int_list: [1, 2]
                  nested: { int: 1, int_list: [1, 2] }
                  nested_list: [
                    { int: 1, int_list: [1, 2] }
                    { int: 2, int_list: [1, 2] }
                  ]
                }
                comp_list: {
                  int: 1
                  int_list: [1, 2]
                  nested: { int: 1, int_list: [1, 2] }
                  nested_list: [
                    { int: 1, int_list: [1, 2] }
                    { int: 2, int_list: [1, 2] }
                  ]
                }
              }
            }
          ) {
            id
          }
        }
        "#),
          @r###"{"data":{"findUniqueTest":{"id":1}}}"###
        );

        Ok(())
    }

    // fn partial_composite_uniq_idx_schema() -> String {
    //     let schema = indoc! {
    //         r#"model Test {
    //           #id(id, Int, @id)
    //           int       Int
    //           comp      Composite   @unique
    //           comp_list Composite[] @unique

    //           @@unique([comp.int])
    //           @@unique([comp.int, comp.int_list])
    //           @@unique([comp.int, comp.int_list, comp.nested])
    //           @@unique([comp.int, comp.int_list, comp.nested, comp.nested.int])
    //           @@unique([comp.int, comp.int_list, comp.nested, comp.nested.int, comp.nested.int_list])
    //           @@unique([comp.int, comp.int_list, comp.nested, comp.nested.int, comp.nested.int_list, comp.nested_list])
    //           @@unique([comp.int, comp.int_list, comp.nested, comp.nested.int, comp.nested.int_list, comp.nested_list, comp.nested_list.int])
    //           @@unique([comp.int, comp.int_list, comp.nested, comp.nested.int, comp.nested.int_list, comp.nested_list, comp.nested_list.int, comp.nested_list.int_list])

    //           @@unique([comp_list.int])
    //           @@unique([comp_list.int, comp_list.int_list])
    //           @@unique([comp_list.int, comp_list.int_list, comp_list.nested])
    //           @@unique([comp_list.int, comp_list.int_list, comp_list.nested, comp_list.nested.int])
    //           @@unique([comp_list.int, comp_list.int_list, comp_list.nested, comp_list.nested.int, comp_list.nested.int_list])
    //           @@unique([comp_list.int, comp_list.int_list, comp_list.nested, comp_list.nested.int, comp_list.nested.int_list, comp_list.nested_list])
    //           @@unique([comp_list.int, comp_list.int_list, comp_list.nested, comp_list.nested.int, comp_list.nested.int_list, comp_list.nested_list, comp_list.nested_list.int], map: "mix_a")
    //           @@unique([comp_list.int, comp_list.int_list, comp_list.nested, comp_list.nested.int, comp_list.nested.int_list, comp_list.nested_list, comp_list.nested_list.int, comp_list.nested_list.int_list], map: "mix_b")
    //       }

    //       type Composite {
    //         int         Int
    //         int_list    Int[]
    //         nested      NestedComposite
    //         nested_list NestedComposite[]
    //       }

    //       type NestedComposite {
    //         int      Int
    //         int_list Int[]
    //       }
    //       "#
    //     };

    //     schema.to_owned()
    // }

    // #[connector_test(schema(partial_composite_uniq_idx_schema))]
    // async fn partial_compound_composite(runner: Runner) -> TestResult<()> {
    //     run_query!(
    //         runner,
    //         r#"mutation {
    //           createManyTest(
    //             data: [
    //               {
    //                 id: 1
    //                 int: 1
    //                 comp: {
    //                   int: 1
    //                   int_list: [1, 2]
    //                   nested: { int: 1, int_list: [1, 2] }
    //                   nested_list: [
    //                     { int: 1, int_list: [1, 2] }
    //                     { int: 2, int_list: [1, 2] }
    //                   ]
    //                 }
    //                 comp_list: [
    //                   {
    //                     int: 1
    //                     int_list: [1, 2]
    //                     nested: { int: 1, int_list: [1, 2] }
    //                     nested_list: [
    //                       { int: 1, int_list: [1, 2] }
    //                       { int: 2, int_list: [1, 2] }
    //                     ]
    //                   }
    //                   {
    //                     int: 2
    //                     int_list: [1, 2]
    //                     nested: { int: 1, int_list: [1, 2] }
    //                     nested_list: [
    //                       { int: 1, int_list: [1, 2] }
    //                       { int: 2, int_list: [1, 2] }
    //                     ]
    //                   }
    //                 ]
    //               }
    //               {
    //                 id: 2
    //                 int: 1
    //                 comp: {
    //                   int: 2
    //                   int_list: [1, 2]
    //                   nested: { int: 1, int_list: [1, 2] }
    //                   nested_list: [
    //                     { int: 1, int_list: [1, 2] }
    //                     { int: 2, int_list: [1, 2] }
    //                   ]
    //                 }
    //                 comp_list: [
    //                   {
    //                     int: 3
    //                     int_list: [1, 2]
    //                     nested: { int: 1, int_list: [1, 2] }
    //                     nested_list: [
    //                       { int: 1, int_list: [1, 2] }
    //                       { int: 2, int_list: [1, 2] }
    //                     ]
    //                   }
    //                   {
    //                     int: 4
    //                     int_list: [1, 2]
    //                     nested: { int: 1, int_list: [1, 2] }
    //                     nested_list: [
    //                       { int: 1, int_list: [1, 2] }
    //                       { int: 2, int_list: [1, 2] }
    //                     ]
    //                   }
    //                 ]
    //               }
    //             ]
    //           ) {
    //             count
    //           }
    //         }
    //         "#
    //     );

    //     // // Partial scalar unique index
    //     // insta::assert_snapshot!(
    //     //   run_query!(&runner, r#""#),
    //     //   @r###""###
    //     // );

    //     Ok(())
    // }
}
