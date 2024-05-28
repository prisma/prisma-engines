use crate::common::expect;

fn expect_errors(schemas: &[[&'static str; 2]], expectation: expect_test::Expect) {
    let out = psl::validate_multi_file(
        &schemas
            .iter()
            .map(|[file_name, contents]| ((*file_name).into(), (*contents).into()))
            .collect::<Vec<_>>(),
    );

    let actual = out.render_own_diagnostics();
    expectation.assert_eq(&actual)
}

#[test]
fn multi_file_errors_single_file() {
    let files: &[[&'static str; 2]] = &[["a.prisma", "meow"]];

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4ma.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmeow[0m
        [1;94m   | [0m
    "#]];
    expect_errors(files, expected);
}

#[test]
fn multi_file_errors_two_files() {
    let files: &[[&'static str; 2]] = &[
        ["a.prisma", "meow"],
        ["b.prisma", "woof woof"],
        ["c.prisma", "choo choo"],
    ];

    let expected = expect![[r#"
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4ma.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mmeow[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4mb.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mwoof woof[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.[0m
          [1;94m-->[0m  [4mc.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mchoo choo[0m
        [1;94m   | [0m
    "#]];
    expect_errors(files, expected);
}

#[test]
fn multi_file_errors_relation() {
    let files: &[[&'static str; 2]] = &[
        [
            "b.prisma",
            r#"
generator client {
    provider = "prisma-client-js"
}

model Post {
    id Int @id
    test String @db.Text
    user_id Int
    user User @relation(fields: [user_id], references: [id])
}
"#,
        ],
        [
            "a.prisma",
            r#"
datasource db {
    provider = "postgresql"
    url = env("TEST_DATABASE_URL")
}

model User {
    id Int @id
    test String @db.FunnyText
    post_id Int @unique
    post Post
}

"#,
        ],
    ];

    let expected = expect![[r#"
        [1;91merror[0m: [1mNative type FunnyText is not supported for postgresql connector.[0m
          [1;94m-->[0m  [4ma.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m    id Int @id
        [1;94m 9 | [0m    test String [1;91m@db.FunnyText[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": A one-to-one relation must use unique fields on the defining side. Either add an `@unique` attribute to the field `user_id`, or change the relation to one-to-many.[0m
          [1;94m-->[0m  [4mb.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m    user_id Int
        [1;94m10 | [0m    [1;91muser User @relation(fields: [user_id], references: [id])[0m
        [1;94m11 | [0m}
        [1;94m   | [0m
    "#]];
    expect_errors(files, expected);
}
