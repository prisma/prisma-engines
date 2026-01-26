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
        .assert_where_clause("status = 'active'");
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
        .assert_where_clause("status = 'active'");
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
        .assert_where_clause("status IS NOT NULL");
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
        .assert_where_clause("status = 'active'");
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
        .assert_where_clause("status = 'active'");
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
        .assert_where_clause("status = 'active'");
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
        .assert_where_clause("\"active\" = true");
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
        .assert_where_clause("\"deleted\" = false");
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
        .assert_where_clause("\"deletedAt\" IS NULL");
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
        .assert_where_clause("\"deletedAt\" IS NOT NULL");
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
        .assert_where_clause("\"status\" = 'active'");
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
        .assert_where_clause("\"priority\" = 1");
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
        .assert_where_clause("\"active\" = true AND \"deletedAt\" IS NULL");
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
        .assert_where_clause("\"active\" = true");
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
        .assert_where_clause("\"status\" != 'deleted'");
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
        .assert_where_clause("status = 'active'");
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
        .assert_where_clause("\"active\" = true");
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
        .assert_where_clause("\"status\" = 'it''s active'");
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
        .assert_where_clause("\"active\" = true");
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
        .assert_where_clause("\"active\" != true");
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
        .assert_where_clause("\"active\" != false");
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
        .assert_where_clause("\"priority\" = -1");
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
        .assert_where_clause("\"score\" = 1.5");
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
