use query_engine_tests::*;

#[test_suite(schema(schema))]
mod many_fields_in_related_table {
    use indoc::indoc;

    fn schema() -> String {
        indoc! {r#"
            model A {
                #id(id, Int, @id)
                field1  Int
                field2  Int
                field3  Int
                field4  Int
                field5  Int
                field6  Int
                field7  Int
                field8  Int
                field9  Int
                field10 Int
                field11 Int
                field12 Int
                field13 Int
                field14 Int
                field15 Int
                field16 Int
                field17 Int
                field18 Int
                field19 Int
                field20 Int
                field21 Int
                field22 Int
                field23 Int
                field24 Int
                field25 Int
                field26 Int
                field27 Int
                field28 Int
                field29 Int
                field30 Int
                field31 Int
                field32 Int
                field33 Int
                field34 Int
                field35 Int
                field36 Int
                field37 Int
                field38 Int
                field39 Int
                field40 Int
                field41 Int
                field42 Int
                field43 Int
                field44 Int
                field45 Int
                field46 Int
                field47 Int
                field48 Int
                field49 Int
                field50 Int
                field51 Int
                b_id    Int
                b       B      @relation(fields: [b_id], references: [id])
                c       C[]
            }

            model B {
                #id(id, Int, @id)
                a A[]
            }

            model C {
                #id(id, Int, @id)
                a_id Int
                a    A   @relation(fields: [a_id], references: [id])
            }
        "#}
        .to_owned()
    }

    #[connector_test]
    async fn query_53_fields_through_relation(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(runner, r#"
                mutation {
                    createOneB(
                        data: {
                            id: 1,
                            a: {
                                create: {
                                    id: 1,
                                    field1: 0,
                                    field2: 0,
                                    field3: 0,
                                    field4: 0,
                                    field5: 0,
                                    field6: 0,
                                    field7: 0,
                                    field8: 0,
                                    field9: 0,
                                    field10: 0,
                                    field11: 0,
                                    field12: 0,
                                    field13: 0,
                                    field14: 0,
                                    field15: 0,
                                    field16: 0,
                                    field17: 0,
                                    field18: 0,
                                    field19: 0,
                                    field20: 0,
                                    field21: 0,
                                    field22: 0,
                                    field23: 0,
                                    field24: 0,
                                    field25: 0,
                                    field26: 0,
                                    field27: 0,
                                    field28: 0,
                                    field29: 0,
                                    field30: 0,
                                    field31: 0,
                                    field32: 0,
                                    field33: 0,
                                    field34: 0,
                                    field35: 0,
                                    field36: 0,
                                    field37: 0,
                                    field38: 0,
                                    field39: 0,
                                    field40: 0,
                                    field41: 0,
                                    field42: 0,
                                    field43: 0,
                                    field44: 0,
                                    field45: 0,
                                    field46: 0,
                                    field47: 0,
                                    field48: 0,
                                    field49: 0,
                                    field50: 0,
                                    field51: 0,
                                    c: {
                                        create: {
                                            id: 1
                                        }
                                    }
                                }
                            }
                        }
                    ) {
                        id
                        a {
                            id
                            field1
                            field2
                            field3
                            field4
                            field5
                            field6
                            field7
                            field8
                            field9
                            field10
                            field11
                            field12
                            field13
                            field14
                            field15
                            field16
                            field17
                            field18
                            field19
                            field20
                            field21
                            field22
                            field23
                            field24
                            field25
                            field26
                            field27
                            field28
                            field29
                            field30
                            field31
                            field32
                            field33
                            field34
                            field35
                            field36
                            field37
                            field38
                            field39
                            field40
                            field41
                            field42
                            field43
                            field44
                            field45
                            field46
                            field47
                            field48
                            field49
                            field50
                            field51
                            c {
                                id
                            }
                        }
                    }
                }
            "#),
            @r###"{"data":{"createOneB":{"id":1,"a":[{"id":1,"field1":0,"field2":0,"field3":0,"field4":0,"field5":0,"field6":0,"field7":0,"field8":0,"field9":0,"field10":0,"field11":0,"field12":0,"field13":0,"field14":0,"field15":0,"field16":0,"field17":0,"field18":0,"field19":0,"field20":0,"field21":0,"field22":0,"field23":0,"field24":0,"field25":0,"field26":0,"field27":0,"field28":0,"field29":0,"field30":0,"field31":0,"field32":0,"field33":0,"field34":0,"field35":0,"field36":0,"field37":0,"field38":0,"field39":0,"field40":0,"field41":0,"field42":0,"field43":0,"field44":0,"field45":0,"field46":0,"field47":0,"field48":0,"field49":0,"field50":0,"field51":0,"c":[{"id":1}]}]}}}"###
        );

        Ok(())
    }
}
