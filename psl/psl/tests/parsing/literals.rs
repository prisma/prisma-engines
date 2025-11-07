use crate::common::*;

#[test]
fn strings_with_quotes_are_unescaped() {
    let input = indoc! {r#"
        model Category {
          id   String @id
          name String @default("a \" b\"c d")
        }
    "#};

    psl::parse_schema_without_extensions(input)
        .unwrap()
        .assert_has_model("Category")
        .assert_has_scalar_field("name")
        .assert_default_value()
        .assert_string("a \" b\"c d");
}

#[test]
fn strings_with_newlines_are_unescaped() {
    let input = indoc! {r#"
        model Category {
          id   String @id
          name String @default("Jean\nClaude\nVan\nDamme")
        }
    "#};

    psl::parse_schema_without_extensions(input)
        .unwrap()
        .assert_has_model("Category")
        .assert_has_scalar_field("name")
        .assert_default_value()
        .assert_string("Jean\nClaude\nVan\nDamme");
}

#[test]
fn strings_with_escaped_unicode_codepoints_are_unescaped() {
    let input = indoc! {r#"
        model Category {
          id   String @id
          name String @default("mfw \u56e7 - \u56E7 ^^")
          // Escaped UTF-16 with surrogate pair (rolling eyes emoji).
          nameUtf16 String @default("oh my \ud83d\ude44...")
        }
    "#};

    let dml = psl::parse_schema_without_extensions(input).unwrap();
    let cat = dml.assert_has_model("Category");

    cat.assert_has_scalar_field("name")
        .assert_default_value()
        .assert_string("mfw 囧 - 囧 ^^");

    cat.assert_has_scalar_field("nameUtf16")
        .assert_default_value()
        .assert_string("oh my 🙄...");
}

#[test]
fn string_literals_with_invalid_unicode_escapes() {
    let input = indoc!(
        r#"
        model Category {
          id   String @id
          name String @default("Something \uD802 \ut \u12")
        }"#
    );

    let expectation = expect![[r#"
        [1;91merror[0m: [1mInvalid unicode escape sequence.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id   String @id
        [1;94m 3 | [0m  name String @default("Something [1;91m\uD802[0m \ut \u12")
        [1;94m   | [0m
        [1;91merror[0m: [1mInvalid unicode escape sequence.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id   String @id
        [1;94m 3 | [0m  name String @default("Something \uD802 [1;91m\u[0mt \u12")
        [1;94m   | [0m
        [1;91merror[0m: [1mInvalid unicode escape sequence.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id   String @id
        [1;94m 3 | [0m  name String @default("Something \uD802 \ut [1;91m\u12[0m")
        [1;94m   | [0m
    "#]];

    expect_error(input, &expectation);
}
