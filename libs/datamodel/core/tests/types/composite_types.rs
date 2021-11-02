use datamodel::{parse_schema, parse_schema_ast};
use expect_test::expect;
use indoc::indoc;

#[test]
fn composite_types_are_parsed_without_error() {
    let datamodel = r#"
        type Address {
            name String?
            street String
            number Int
            zipCode Int?
        }

        model User {
            id Int @id
            address Address?
        }
    "#;

    let expected = expect![[r#"
        SchemaAst {
            tops: [
                CompositeType(
                    CompositeType {
                        name: Identifier {
                            name: "Address",
                            span: Span {
                                start: 14,
                                end: 21,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 41,
                                            end: 47,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "name",
                                    span: Span {
                                        start: 36,
                                        end: 40,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 36,
                                    end: 49,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 68,
                                            end: 74,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "street",
                                    span: Span {
                                        start: 61,
                                        end: 67,
                                    },
                                },
                                arity: Required,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 61,
                                    end: 75,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Int",
                                        span: Span {
                                            start: 94,
                                            end: 97,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "number",
                                    span: Span {
                                        start: 87,
                                        end: 93,
                                    },
                                },
                                arity: Required,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 87,
                                    end: 98,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Int",
                                        span: Span {
                                            start: 118,
                                            end: 121,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "zipCode",
                                    span: Span {
                                        start: 110,
                                        end: 117,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 110,
                                    end: 123,
                                },
                                is_commented_out: false,
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 9,
                            end: 132,
                        },
                    },
                ),
                Model(
                    Model {
                        name: Identifier {
                            name: "User",
                            span: Span {
                                start: 148,
                                end: 152,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Int",
                                        span: Span {
                                            start: 170,
                                            end: 173,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "id",
                                    span: Span {
                                        start: 167,
                                        end: 169,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "id",
                                            span: Span {
                                                start: 175,
                                                end: 177,
                                            },
                                        },
                                        arguments: [],
                                        span: Span {
                                            start: 175,
                                            end: 177,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 167,
                                    end: 178,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Address",
                                        span: Span {
                                            start: 198,
                                            end: 205,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "address",
                                    span: Span {
                                        start: 190,
                                        end: 197,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 190,
                                    end: 207,
                                },
                                is_commented_out: false,
                            },
                        ],
                        attributes: [],
                        documentation: None,
                        span: Span {
                            start: 142,
                            end: 216,
                        },
                        commented_out: false,
                    },
                ),
            ],
        }
    "#]];
    let found = parse_schema_ast(datamodel).unwrap();

    expected.assert_debug_eq(&found);
}

#[test]
fn composite_types_cannot_have_block_attributes() {
    let datamodel = r#"
        type Address {
            name String?

            @@unique([name])
        }
    "#;

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Composite types cannot have block attributes.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m            [1;91m@@unique([name])[0m
        [1;94m 6 | [0m        }
        [1;94m   | [0m
    "#]];
    let found = parse_schema_ast(datamodel)
        .unwrap_err()
        .to_pretty_string("schema.prisma", datamodel);

    expected.assert_eq(&found);
}

#[test]
fn composite_types_can_nest() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["mongoDb"]
        }

        type Address {
            name String?
            secondaryAddress Address?
        }
    "#;

    assert!(parse_schema(schema).is_ok());
}

#[test]
fn required_cycles_to_self_are_not_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        type Address {
          name String?
          secondaryAddress Address
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `secondaryAddress` in composite type `Address`: The type is the same as the parent and causes an endless cycle. Please change the field to be either optional or a list.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  name String?
        [1;94m13 | [0m  [1;91msecondaryAddress Address[0m
        [1;94m14 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&parse_schema(datamodel).unwrap_err());
}

#[test]
fn list_cycles_to_self_are_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        type Address {
          name String?
          secondaryAddresses Address[]
        }
    "#};

    assert!(parse_schema(datamodel).is_ok())
}

#[test]
fn required_cycles_are_not_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        type PostCode {
          code Int
        }

        type Address {
          name String?
          city City
          code PostCode
        }

        type City {
          name         String?
          worldAddress Address
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `worldAddress` in composite type `City`: The types cause an endless cycle in the path `City` → `Address` → `City`. Please change one of the fields to be either optional or a list to break the cycle.[0m
          [1;94m-->[0m  [4mschema.prisma:23[0m
        [1;94m   | [0m
        [1;94m22 | [0m  name         String?
        [1;94m23 | [0m  [1;91mworldAddress Address[0m
        [1;94m24 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `city` in composite type `Address`: The types cause an endless cycle in the path `Address` → `City` → `Address`. Please change one of the fields to be either optional or a list to break the cycle.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  name String?
        [1;94m17 | [0m  [1;91mcity City[0m
        [1;94m18 | [0m  code PostCode
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&parse_schema(datamodel).unwrap_err());
}

#[test]
fn cycles_broken_with_an_optional_are_allowed() {
    let datamodel = indoc! {r#"
        datasource db {
          provider = "mongodb"
          url = "mongodb://"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        type PostCode {
          code Int
        }

        type Address {
          name String?
          city City
          code PostCode
        }

        type City {
          name         String?
          worldAddress Address?
        }
    "#};

    assert!(parse_schema(datamodel).is_ok());
}
