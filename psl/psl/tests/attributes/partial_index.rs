use psl::parser_database::{WhereCondition, WhereValue};

use crate::{Provider, common::*, with_header};

#[test]
fn partial_unique_index_on_postgres() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw("status = 'active'"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_raw_where_clause("status = 'active'");
}

#[test]
fn partial_unique_index_with_name_on_postgres() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], name: "email_active_unique", where: raw("status = 'active'"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_name("email_active_unique")
        .assert_raw_where_clause("status = 'active'");
}

#[test]
fn partial_index_on_postgres() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@index([email], where: raw("status IS NOT NULL"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["email"])
        .assert_raw_where_clause("status IS NOT NULL");
}

#[test]
fn partial_unique_index_on_sqlite() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw("status = 'active'"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Sqlite, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_raw_where_clause("status = 'active'");
}

#[test]
fn partial_index_on_sqlite() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@index([email], where: raw("status = 'active'"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Sqlite, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["email"])
        .assert_raw_where_clause("status = 'active'");
}

#[test]
fn partial_index_not_supported_on_mysql() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw("status = 'active'"))
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Mysql, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": Partial indexes (with a `where` clause) are not supported by the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    @@unique([email], [1;91mwhere: raw("status = 'active'")[0m)
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_requires_raw_function() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: "status = 'active'")
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mExpected a function value, but received string value `"status = 'active'"`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    @@unique([email], where: [1;91m"status = 'active'"[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@unique": The `where` argument must be either a raw() function call or an object literal, e.g. `where: raw("status = 'active'")` or `where: { active: true }`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    [1;91m@@unique([email], where: "status = 'active'")[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_raw_requires_string_argument() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw())
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The `where` argument must be a raw() function with a string argument, e.g. `where: raw("status = 'active'")`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    [1;91m@@unique([email], where: raw())[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_cannot_have_empty_predicate() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw(""))
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The `where` argument cannot contain an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    [1;91m@@unique([email], where: raw(""))[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn regular_index_has_no_where_clause() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String

            @@unique([email])
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_no_where_clause();
}

#[test]
fn compound_partial_unique_index() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String
            status    String

            @@unique([firstName, lastName], where: raw("status = 'active'"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["firstName", "lastName"])
        .assert_raw_where_clause("status = 'active'");
}

#[test]
fn partial_unique_index_with_object_syntax_boolean_true() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@unique([email], where: { active: true })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("active", WhereCondition::Equals(WhereValue::Boolean(true)))]);
}

#[test]
fn partial_unique_index_with_object_syntax_boolean_false() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            deleted   Boolean

            @@unique([email], where: { deleted: false })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("deleted", WhereCondition::Equals(WhereValue::Boolean(false)))]);
}

#[test]
fn partial_unique_index_with_object_syntax_null() {
    let dml = indoc! {r#"
        model User {
            id        Int       @id
            email     String
            deletedAt DateTime?

            @@unique([email], where: { deletedAt: null })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("deletedAt", WhereCondition::IsNull)]);
}

#[test]
fn partial_unique_index_with_object_syntax_not_null() {
    let dml = indoc! {r#"
        model User {
            id        Int       @id
            email     String
            deletedAt DateTime?

            @@unique([email], where: { deletedAt: { not: null } })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("deletedAt", WhereCondition::IsNotNull)]);
}

#[test]
fn partial_unique_index_with_object_syntax_string_value() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: { status: "active" })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("status", WhereCondition::Equals(WhereValue::String("active".into())))]);
}

#[test]
fn partial_unique_index_with_object_syntax_number_value() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            priority  Int

            @@unique([email], where: { priority: 1 })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("priority", WhereCondition::Equals(WhereValue::Number("1".into())))]);
}

#[test]
fn partial_unique_index_with_object_syntax_multiple_conditions() {
    let dml = indoc! {r#"
        model User {
            id        Int       @id
            email     String
            active    Boolean
            deletedAt DateTime?

            @@unique([email], where: { active: true, deletedAt: null })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[
            ("active", WhereCondition::Equals(WhereValue::Boolean(true))),
            ("deletedAt", WhereCondition::IsNull),
        ]);
}

#[test]
fn partial_index_with_object_syntax_on_sqlite() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@index([email], where: { active: true })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Sqlite, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["email"])
        .assert_where_object(&[("active", WhereCondition::Equals(WhereValue::Boolean(true)))]);
}

#[test]
fn partial_index_with_object_syntax_not_supported_on_mysql() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@unique([email], where: { active: true })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Mysql, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": Partial indexes (with a `where` clause) are not supported by the current connector.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    @@unique([email], [1;91mwhere: { active: true }[0m)
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_with_empty_object_is_invalid() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String

            @@unique([email], where: {})
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": The `where` argument cannot be an empty object.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([email], where: {})[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_with_invalid_object_value() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: { status: invalid })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Invalid value 'invalid' in where clause. Expected true, false, null, a string, a number, or an object like { not: null }.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    [1;91m@@unique([email], where: { status: invalid })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_with_not_string_value() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: { status: { not: "deleted" } })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[(
            "status",
            WhereCondition::NotEquals(WhereValue::String("deleted".into())),
        )]);
}

#[test]
fn partial_unique_index_on_cockroachdb() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw("status = 'active'"))
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Cockroach, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_raw_where_clause("status = 'active'");
}

#[test]
fn partial_index_with_object_syntax_on_cockroachdb() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@index([email], where: { active: true })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Cockroach, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["email"])
        .assert_where_object(&[("active", WhereCondition::Equals(WhereValue::Boolean(true)))]);
}

#[test]
fn partial_index_with_special_characters_in_string() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: { status: "it's active" })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[(
            "status",
            WhereCondition::Equals(WhereValue::String("it's active".into())),
        )]);
}

#[test]
fn partial_index_object_syntax_on_index() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@index([email], where: { active: true })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_index_on_fields(&["email"])
        .assert_where_object(&[("active", WhereCondition::Equals(WhereValue::Boolean(true)))]);
}

#[test]
fn partial_index_with_not_true() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@unique([email], where: { active: { not: true } })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("active", WhereCondition::NotEquals(WhereValue::Boolean(true)))]);
}

#[test]
fn partial_index_with_not_false() {
    let dml = indoc! {r#"
        model User {
            id        Int     @id
            email     String
            active    Boolean

            @@unique([email], where: { active: { not: false } })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("active", WhereCondition::NotEquals(WhereValue::Boolean(false)))]);
}

#[test]
fn partial_index_with_negative_number() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            priority  Int

            @@unique([email], where: { priority: -1 })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("priority", WhereCondition::Equals(WhereValue::Number("-1".into())))]);
}

#[test]
fn partial_index_with_decimal_number() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            score     Float

            @@unique([email], where: { score: 1.5 })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["email"])
        .assert_where_object(&[("score", WhereCondition::Equals(WhereValue::Number("1.5".into())))]);
}

#[test]
fn partial_index_nested_object_with_multiple_keys_is_invalid() {
    let dml = indoc! {r#"
        model User {
            id        Int       @id
            email     String
            deletedAt DateTime?

            @@unique([email], where: { deletedAt: { not: null, eq: "value" } })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Nested object in where clause must have exactly one key. Use `{ not: null }` or `{ not: "value" }`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    [1;91m@@unique([email], where: { deletedAt: { not: null, eq: "value" } })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_nested_object_with_unknown_key_is_invalid() {
    let dml = indoc! {r#"
        model User {
            id        Int       @id
            email     String
            deletedAt DateTime?

            @@unique([email], where: { deletedAt: { eq: null } })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Unknown key 'eq' in nested where clause object. Only 'not' is supported.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    [1;91m@@unique([email], where: { deletedAt: { eq: null } })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_requires_preview_feature() {
    let dml = indoc! {r#"
        model User {
            id        Int    @id
            email     String
            status    String

            @@unique([email], where: raw("status = 'active'"))
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &[]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@@unique": Partial indexes are a preview feature. Add "partialIndexes" to previewFeatures in your generator block.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m
        [1;94m15 | [0m    @@unique([email], [1;91mwhere: raw("status = 'active'")[0m)
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_uses_database_name() {
    let dml = indoc! {r#"
        model User {
            id       Int     @id
            isActive Boolean @map("is_active")

            @@unique([id], where: { isActive: true })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["id"])
        .assert_where_object(&[("is_active", WhereCondition::Equals(WhereValue::Boolean(true)))]);
}

#[test]
fn partial_index_object_syntax_rejects_relation_field() {
    let dml = indoc! {r#"
        model User {
            id    Int    @id
            posts Post[]

            @@unique([id], where: { posts: true })
        }

        model Post {
            id     Int  @id
            userId Int
            user   User @relation(fields: [userId], references: [id])
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Field 'posts' is a relation field. Only scalar fields can be used in the where clause.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { posts: true })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_boolean_value_for_string_field() {
    let dml = indoc! {r#"
        model User {
            id     Int    @id
            status String

            @@unique([id], where: { status: true })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'status' is of type String, but the value is Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { status: true })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_string_value_for_boolean_field() {
    let dml = indoc! {r#"
        model User {
            id     Int     @id
            active Boolean

            @@unique([id], where: { active: "yes" })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'active' is of type Boolean, but the value is a String.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { active: "yes" })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_number_value_for_string_field() {
    let dml = indoc! {r#"
        model User {
            id     Int    @id
            status String

            @@unique([id], where: { status: 123 })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'status' is of type String, but the value is a Number.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { status: 123 })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_accepts_null_for_any_field() {
    let dml = indoc! {r#"
        model User {
            id     Int     @id
            name   String?
            active Boolean?
            count  Int?

            @@unique([id], where: { name: null })
            @@index([id], where: { active: { not: null } })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"])).unwrap();
}

#[test]
fn partial_index_object_syntax_accepts_number_for_int_field() {
    let dml = indoc! {r#"
        model User {
            id       Int @id
            priority Int

            @@unique([id], where: { priority: 1 })
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["id"])
        .assert_where_object(&[("priority", WhereCondition::Equals(WhereValue::Number("1".into())))]);
}

#[test]
fn partial_index_object_syntax_rejects_type_mismatch_in_not() {
    let dml = indoc! {r#"
        model User {
            id     Int    @id
            status String

            @@unique([id], where: { status: { not: true } })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'status' is of type String, but the value is Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { status: { not: true } })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_number_for_boolean_field() {
    let dml = indoc! {r#"
        model User {
            id     Int     @id
            active Boolean

            @@unique([id], where: { active: 1 })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'active' is of type Boolean, but the value is a Number.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { active: 1 })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_boolean_for_int_field() {
    let dml = indoc! {r#"
        model User {
            id    Int @id
            count Int

            @@unique([id], where: { count: true })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'count' is of type Int, but the value is Boolean.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { count: true })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_string_for_int_field() {
    let dml = indoc! {r#"
        model User {
            id    Int @id
            count Int

            @@unique([id], where: { count: "high" })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'count' is of type Int, but the value is a String.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { count: "high" })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_accepts_string_for_enum_field() {
    let dml = indoc! {r#"
        model User {
            id     Int    @id
            role   Role

            @@unique([id], where: { role: "ADMIN" })
        }

        enum Role {
            ADMIN
            USER
        }
    "#};

    psl::parse_schema_without_extensions(with_header(dml, Provider::Postgres, &["partialIndexes"]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["id"])
        .assert_where_object(&[("role", WhereCondition::Equals(WhereValue::String("ADMIN".into())))]);
}

#[test]
fn partial_index_object_syntax_rejects_unsupported_type_field() {
    let dml = indoc! {r#"
        model User {
            id   Int                      @id
            geom Unsupported("geometry")

            @@unique([id], where: { geom: "point" })
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Field 'geom' is an unsupported type and cannot be used in the object syntax of a where clause. Use raw() instead.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { geom: "point" })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}

#[test]
fn partial_index_object_syntax_rejects_boolean_for_enum_field() {
    let dml = indoc! {r#"
        model User {
            id   Int  @id
            role Role

            @@unique([id], where: { role: true })
        }

        enum Role {
            ADMIN
            USER
        }
    "#};

    let err = parse_unwrap_err(&with_header(dml, Provider::Postgres, &["partialIndexes"]));
    let expected = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@unique": Type mismatch: field 'role' is an Enum and only accepts String values in the where clause.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m
        [1;94m14 | [0m    [1;91m@@unique([id], where: { role: true })[0m
        [1;94m   | [0m
    "#]];
    expected.assert_eq(&err);
}
