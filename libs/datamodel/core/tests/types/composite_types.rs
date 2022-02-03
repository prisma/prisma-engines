use datamodel::{parse_schema, parse_schema_ast};
use expect_test::expect;
use indoc::indoc;

#[test]
fn composite_types_are_parsed_without_error() {
    let datamodel = r#"
        datasource db{
            provider = "mongodb"
            url = "mongo+srv:/...."
        }
    
        generator client {
          provider        = "prisma-client-js"
          previewFeatures = ["mongoDb"]
        }

        type Address {
            name String?
            street String @db.ObjectId
        }

        model User {
            id  String @id @default(auto()) @map("_id") @db.ObjectId
            address Address?
        }
    "#;

    let expected_ast = expect![[r#"
        SchemaAst {
            tops: [
                Source(
                    SourceConfig {
                        name: Identifier {
                            name: "db",
                            span: Span {
                                start: 20,
                                end: 22,
                            },
                        },
                        properties: [
                            ConfigBlockProperty {
                                name: Identifier {
                                    name: "provider",
                                    span: Span {
                                        start: 36,
                                        end: 44,
                                    },
                                },
                                value: StringValue(
                                    "mongodb",
                                    Span {
                                        start: 47,
                                        end: 56,
                                    },
                                ),
                                span: Span {
                                    start: 36,
                                    end: 57,
                                },
                            },
                            ConfigBlockProperty {
                                name: Identifier {
                                    name: "url",
                                    span: Span {
                                        start: 69,
                                        end: 72,
                                    },
                                },
                                value: StringValue(
                                    "mongo+srv:/....",
                                    Span {
                                        start: 75,
                                        end: 92,
                                    },
                                ),
                                span: Span {
                                    start: 69,
                                    end: 93,
                                },
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 9,
                            end: 102,
                        },
                    },
                ),
                Generator(
                    GeneratorConfig {
                        name: Identifier {
                            name: "client",
                            span: Span {
                                start: 126,
                                end: 132,
                            },
                        },
                        properties: [
                            ConfigBlockProperty {
                                name: Identifier {
                                    name: "provider",
                                    span: Span {
                                        start: 145,
                                        end: 153,
                                    },
                                },
                                value: StringValue(
                                    "prisma-client-js",
                                    Span {
                                        start: 163,
                                        end: 181,
                                    },
                                ),
                                span: Span {
                                    start: 145,
                                    end: 182,
                                },
                            },
                            ConfigBlockProperty {
                                name: Identifier {
                                    name: "previewFeatures",
                                    span: Span {
                                        start: 192,
                                        end: 207,
                                    },
                                },
                                value: Array(
                                    [
                                        StringValue(
                                            "mongoDb",
                                            Span {
                                                start: 211,
                                                end: 220,
                                            },
                                        ),
                                    ],
                                    Span {
                                        start: 210,
                                        end: 221,
                                    },
                                ),
                                span: Span {
                                    start: 192,
                                    end: 222,
                                },
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 116,
                            end: 231,
                        },
                    },
                ),
                CompositeType(
                    CompositeType {
                        name: Identifier {
                            name: "Address",
                            span: Span {
                                start: 246,
                                end: 253,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 273,
                                            end: 279,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "name",
                                    span: Span {
                                        start: 268,
                                        end: 272,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 268,
                                    end: 281,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 300,
                                            end: 306,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "street",
                                    span: Span {
                                        start: 293,
                                        end: 299,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "db.ObjectId",
                                            span: Span {
                                                start: 308,
                                                end: 319,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 308,
                                            end: 319,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 293,
                                    end: 320,
                                },
                                is_commented_out: false,
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 241,
                            end: 329,
                        },
                    },
                ),
                Model(
                    Model {
                        name: Identifier {
                            name: "User",
                            span: Span {
                                start: 345,
                                end: 349,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 368,
                                            end: 374,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "id",
                                    span: Span {
                                        start: 364,
                                        end: 366,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "id",
                                            span: Span {
                                                start: 376,
                                                end: 378,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 376,
                                            end: 378,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "default",
                                            span: Span {
                                                start: 380,
                                                end: 387,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [
                                                Argument {
                                                    name: None,
                                                    value: Function(
                                                        "auto",
                                                        ArgumentsList {
                                                            arguments: [],
                                                            empty_arguments: [],
                                                            trailing_comma: None,
                                                        },
                                                        Span {
                                                            start: 388,
                                                            end: 394,
                                                        },
                                                    ),
                                                    span: Span {
                                                        start: 388,
                                                        end: 394,
                                                    },
                                                },
                                            ],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 380,
                                            end: 395,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "map",
                                            span: Span {
                                                start: 397,
                                                end: 400,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [
                                                Argument {
                                                    name: None,
                                                    value: StringValue(
                                                        "_id",
                                                        Span {
                                                            start: 401,
                                                            end: 406,
                                                        },
                                                    ),
                                                    span: Span {
                                                        start: 401,
                                                        end: 406,
                                                    },
                                                },
                                            ],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 397,
                                            end: 407,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "db.ObjectId",
                                            span: Span {
                                                start: 409,
                                                end: 420,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 409,
                                            end: 420,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 364,
                                    end: 421,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Address",
                                        span: Span {
                                            start: 441,
                                            end: 448,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "address",
                                    span: Span {
                                        start: 433,
                                        end: 440,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 433,
                                    end: 450,
                                },
                                is_commented_out: false,
                            },
                        ],
                        attributes: [],
                        documentation: None,
                        span: Span {
                            start: 339,
                            end: 459,
                        },
                        commented_out: false,
                    },
                ),
            ],
        }
    "#]];

    let found = parse_schema_ast(datamodel).unwrap();
    let (_, datamodel) = parse_schema(datamodel).unwrap();

    let expected_datamodel = expect![[r#"
        Datamodel {
            enums: [],
            models: [
                Model {
                    name: "User",
                    fields: [
                        CompositeField(
                            CompositeField {
                                name: "address",
                                database_name: None,
                                composite_type: "Address",
                                arity: Optional,
                                documentation: None,
                                is_commented_out: false,
                                is_ignored: false,
                                default_value: None,
                            },
                        ),
                        ScalarField(
                            ScalarField {
                                name: "id",
                                field_type: Scalar(
                                    String,
                                    None,
                                    Some(
                                        NativeTypeInstance {
                                            name: "ObjectId",
                                            args: [],
                                            serialized_native_type: String(
                                                "ObjectId",
                                            ),
                                        },
                                    ),
                                ),
                                arity: Required,
                                database_name: Some(
                                    "_id",
                                ),
                                default_value: Some(
                                    DefaultValue::Expression(auto()[]),
                                ),
                                documentation: None,
                                is_generated: false,
                                is_updated_at: false,
                                is_commented_out: false,
                                is_ignored: false,
                            },
                        ),
                    ],
                    documentation: None,
                    database_name: None,
                    indices: [],
                    primary_key: Some(
                        PrimaryKeyDefinition {
                            name: None,
                            db_name: None,
                            fields: [
                                PrimaryKeyField {
                                    name: "id",
                                    sort_order: None,
                                    length: None,
                                },
                            ],
                            defined_on_field: true,
                        },
                    ),
                    is_generated: false,
                    is_commented_out: false,
                    is_ignored: false,
                },
            ],
            composite_types: [
                CompositeType {
                    name: "Address",
                    fields: [
                        CompositeTypeField {
                            name: "name",
                            type: Scalar(
                                String,
                                None,
                                None,
                            ),
                            arity: Optional,
                            database_name: None,
                            documentation: None,
                            default_value: None,
                        },
                        CompositeTypeField {
                            name: "street",
                            type: Scalar(
                                String,
                                None,
                                Some(
                                    NativeTypeInstance {
                                        name: "ObjectId",
                                        args: [],
                                        serialized_native_type: String(
                                            "ObjectId",
                                        ),
                                    },
                                ),
                            ),
                            arity: Required,
                            database_name: None,
                            documentation: None,
                            default_value: None,
                        },
                    ],
                },
            ],
        }
    "#]];

    expected_ast.assert_debug_eq(&found);
    expected_datamodel.assert_debug_eq(&datamodel);
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
