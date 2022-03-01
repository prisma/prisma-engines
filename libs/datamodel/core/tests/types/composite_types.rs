use datamodel::{parse_schema, parse_schema_ast};
use expect_test::expect;
use indoc::indoc;

use crate::{
    common::{CompositeTypeAsserts, DatamodelAsserts},
    with_header, Provider,
};

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
                                start: 122,
                                end: 128,
                            },
                        },
                        properties: [
                            ConfigBlockProperty {
                                name: Identifier {
                                    name: "provider",
                                    span: Span {
                                        start: 141,
                                        end: 149,
                                    },
                                },
                                value: StringValue(
                                    "prisma-client-js",
                                    Span {
                                        start: 159,
                                        end: 177,
                                    },
                                ),
                                span: Span {
                                    start: 141,
                                    end: 178,
                                },
                            },
                            ConfigBlockProperty {
                                name: Identifier {
                                    name: "previewFeatures",
                                    span: Span {
                                        start: 188,
                                        end: 203,
                                    },
                                },
                                value: Array(
                                    [
                                        StringValue(
                                            "mongoDb",
                                            Span {
                                                start: 207,
                                                end: 216,
                                            },
                                        ),
                                    ],
                                    Span {
                                        start: 206,
                                        end: 217,
                                    },
                                ),
                                span: Span {
                                    start: 188,
                                    end: 218,
                                },
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 112,
                            end: 227,
                        },
                    },
                ),
                CompositeType(
                    CompositeType {
                        name: Identifier {
                            name: "Address",
                            span: Span {
                                start: 242,
                                end: 249,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 269,
                                            end: 275,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "name",
                                    span: Span {
                                        start: 264,
                                        end: 268,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 264,
                                    end: 277,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 296,
                                            end: 302,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "street",
                                    span: Span {
                                        start: 289,
                                        end: 295,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "db.ObjectId",
                                            span: Span {
                                                start: 304,
                                                end: 315,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 304,
                                            end: 315,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 289,
                                    end: 316,
                                },
                                is_commented_out: false,
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 237,
                            end: 325,
                        },
                    },
                ),
                Model(
                    Model {
                        name: Identifier {
                            name: "User",
                            span: Span {
                                start: 341,
                                end: 345,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 364,
                                            end: 370,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "id",
                                    span: Span {
                                        start: 360,
                                        end: 362,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "id",
                                            span: Span {
                                                start: 372,
                                                end: 374,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 372,
                                            end: 374,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "default",
                                            span: Span {
                                                start: 376,
                                                end: 383,
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
                                                            start: 384,
                                                            end: 390,
                                                        },
                                                    ),
                                                    span: Span {
                                                        start: 384,
                                                        end: 390,
                                                    },
                                                },
                                            ],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 376,
                                            end: 391,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "map",
                                            span: Span {
                                                start: 393,
                                                end: 396,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [
                                                Argument {
                                                    name: None,
                                                    value: StringValue(
                                                        "_id",
                                                        Span {
                                                            start: 397,
                                                            end: 402,
                                                        },
                                                    ),
                                                    span: Span {
                                                        start: 397,
                                                        end: 402,
                                                    },
                                                },
                                            ],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 393,
                                            end: 403,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "db.ObjectId",
                                            span: Span {
                                                start: 405,
                                                end: 416,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 405,
                                            end: 416,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 360,
                                    end: 417,
                                },
                                is_commented_out: false,
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Address",
                                        span: Span {
                                            start: 437,
                                            end: 444,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "address",
                                    span: Span {
                                        start: 429,
                                        end: 436,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 429,
                                    end: 446,
                                },
                                is_commented_out: false,
                            },
                        ],
                        attributes: [],
                        documentation: None,
                        span: Span {
                            start: 335,
                            end: 455,
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
                            is_commented_out: false,
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
                            is_commented_out: false,
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
fn composite_types_must_have_at_least_one_visible_field() {
    let schema = indoc! {r#"
        type Address {
          // name String?
        }
    "#};

    let datamodel = with_header(schema, Provider::Mongo, &["mongoDb"]);

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: A type must have at least one field defined.[0m
          [1;94m-->[0m  [4mschema.prisma:11[0m
        [1;94m   | [0m
        [1;94m10 | [0m
        [1;94m11 | [0m[1;91mtype Address {[0m
        [1;94m12 | [0m  // name String?
        [1;94m13 | [0m}
        [1;94m   | [0m
    "#]];

    let error = datamodel::parse_schema(&datamodel).map(drop).unwrap_err();

    expected.assert_eq(&error);
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

#[test]
fn unsupported_should_work() {
    let schema = indoc! {r#"
        type A {
          field Unsupported("Unknown")
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &["mongoDb"]);
    let (_, datamodel) = parse_schema(&dml).unwrap();

    datamodel
        .assert_has_composite_type("A")
        .assert_has_unsupported_field("field");
}
