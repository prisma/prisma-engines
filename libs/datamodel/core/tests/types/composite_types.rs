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
                CompositeType(
                    CompositeType {
                        name: Identifier {
                            name: "Address",
                            span: Span {
                                start: 117,
                                end: 124,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 144,
                                            end: 150,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "name",
                                    span: Span {
                                        start: 139,
                                        end: 143,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 139,
                                    end: 152,
                                },
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 171,
                                            end: 177,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "street",
                                    span: Span {
                                        start: 164,
                                        end: 170,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "db.ObjectId",
                                            span: Span {
                                                start: 179,
                                                end: 190,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 178,
                                            end: 190,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 164,
                                    end: 191,
                                },
                            },
                        ],
                        documentation: None,
                        span: Span {
                            start: 112,
                            end: 200,
                        },
                    },
                ),
                Model(
                    Model {
                        name: Identifier {
                            name: "User",
                            span: Span {
                                start: 216,
                                end: 220,
                            },
                        },
                        fields: [
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "String",
                                        span: Span {
                                            start: 239,
                                            end: 245,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "id",
                                    span: Span {
                                        start: 235,
                                        end: 237,
                                    },
                                },
                                arity: Required,
                                attributes: [
                                    Attribute {
                                        name: Identifier {
                                            name: "id",
                                            span: Span {
                                                start: 247,
                                                end: 249,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 246,
                                            end: 250,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "default",
                                            span: Span {
                                                start: 251,
                                                end: 258,
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
                                                            start: 259,
                                                            end: 265,
                                                        },
                                                    ),
                                                    span: Span {
                                                        start: 259,
                                                        end: 265,
                                                    },
                                                },
                                            ],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 250,
                                            end: 266,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "map",
                                            span: Span {
                                                start: 268,
                                                end: 271,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [
                                                Argument {
                                                    name: None,
                                                    value: StringValue(
                                                        "_id",
                                                        Span {
                                                            start: 272,
                                                            end: 277,
                                                        },
                                                    ),
                                                    span: Span {
                                                        start: 272,
                                                        end: 277,
                                                    },
                                                },
                                            ],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 267,
                                            end: 278,
                                        },
                                    },
                                    Attribute {
                                        name: Identifier {
                                            name: "db.ObjectId",
                                            span: Span {
                                                start: 280,
                                                end: 291,
                                            },
                                        },
                                        arguments: ArgumentsList {
                                            arguments: [],
                                            empty_arguments: [],
                                            trailing_comma: None,
                                        },
                                        span: Span {
                                            start: 279,
                                            end: 291,
                                        },
                                    },
                                ],
                                documentation: None,
                                span: Span {
                                    start: 235,
                                    end: 292,
                                },
                            },
                            Field {
                                field_type: Supported(
                                    Identifier {
                                        name: "Address",
                                        span: Span {
                                            start: 312,
                                            end: 319,
                                        },
                                    },
                                ),
                                name: Identifier {
                                    name: "address",
                                    span: Span {
                                        start: 304,
                                        end: 311,
                                    },
                                },
                                arity: Optional,
                                attributes: [],
                                documentation: None,
                                span: Span {
                                    start: 304,
                                    end: 321,
                                },
                            },
                        ],
                        attributes: [],
                        documentation: None,
                        span: Span {
                            start: 210,
                            end: 330,
                        },
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
                        ScalarField(
                            ScalarField {
                                name: "id",
                                field_type: Scalar(
                                    String,
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
                            clustered: None,
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

    let datamodel = with_header(schema, Provider::Mongo, &[]);

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
fn composite_types_can_nest() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
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

        type Address {
          name String?
          secondaryAddress Address
        }
    "#};

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating field `secondaryAddress` in composite type `Address`: The type is the same as the parent and causes an endless cycle. Please change the field to be either optional or a list.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  name String?
        [1;94m 8 | [0m  [1;91msecondaryAddress Address[0m
        [1;94m 9 | [0m}
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
          [1;94m-->[0m  [4mschema.prisma:18[0m
        [1;94m   | [0m
        [1;94m17 | [0m  name         String?
        [1;94m18 | [0m  [1;91mworldAddress Address[0m
        [1;94m19 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `city` in composite type `Address`: The types cause an endless cycle in the path `Address` → `City` → `Address`. Please change one of the fields to be either optional or a list to break the cycle.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0m  name String?
        [1;94m12 | [0m  [1;91mcity City[0m
        [1;94m13 | [0m  code PostCode
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

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let (_, datamodel) = parse_schema(&dml).unwrap();

    datamodel
        .assert_has_composite_type("A")
        .assert_has_unsupported_field("field");
}

#[test]
fn block_level_map_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int

          @@map("foo")
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: The name of a composite type is not persisted in the database, therefore it does not need a mapped database name.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@map("foo")[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn block_level_unique_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int

          @@unique([field])
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: A unique constraint should be defined in the model containing the embed.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@unique([field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn block_level_index_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int

          @@index([field])
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: An index should be defined in the model containing the embed.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@index([field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn block_level_fulltext_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int

          @@fulltext([field])
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: A fulltext index should be defined in the model containing the embed.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@fulltext([field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn block_level_id_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int

          @@id([field])
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: A composite type cannot define an id.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m  [1;91m@@id([field])[0m
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn id_field_attribute_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int @id
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Defining `@id` attribute for a field in a composite type is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype A {
        [1;94m12 | [0m  [1;91mfield Int @id[0m
        [1;94m13 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn unique_field_attribute_not_allowed() {
    let schema = indoc! {r#"
        type A {
          field Int @unique
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Defining `@unique` attribute for a field in a composite type is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:12[0m
        [1;94m   | [0m
        [1;94m11 | [0mtype A {
        [1;94m12 | [0m  [1;91mfield Int @unique[0m
        [1;94m13 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}

#[test]
fn realation_field_attribute_not_allowed() {
    let schema = indoc! {r#"
        type C {
          val String
        }

        type A {
          c C[] @relation("foo")
        }

        model B {
          id Int @id
          a  A
        }
    "#};

    let dml = with_header(schema, crate::Provider::Mongo, &[]);
    let error = datamodel::parse_schema(&dml).map(drop).unwrap_err();

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: Defining `@relation` attribute for a field in a composite type is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0mtype A {
        [1;94m16 | [0m  [1;91mc C[] @relation("foo")[0m
        [1;94m17 | [0m}
        [1;94m   | [0m
    "#]];

    expected.assert_eq(&error);
}
