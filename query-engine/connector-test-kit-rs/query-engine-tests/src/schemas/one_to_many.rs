use indoc::indoc;

/// User <-1---m-> posts
pub fn user_posts() -> String {
    let schema = indoc! {
        r#"model User {
            #id(id, Int, @id)
            first_name String
            last_name  String
            email      String    @unique
            birthday   DateTime?
            posts Post[]
        }

        model Post {
            #id(id, Int, @id)
            title     String
            content   String @default("Wip")
            author_id Int
            author    User   @relation(fields: [author_id], references: [id])
        }"#
    };

    schema.to_owned()
}

/// A <-1?---m-> B
pub fn a1_to_bm_opt() -> String {
    let schema = indoc! {
        r#"model A {
            #id(id, Int, @id)
            many_b B[]
        }

        model B {
            #id(id, Int, @id)
            a_id Int?
            a    A?   @relation(fields: [a_id], references: [id])
        }"#
    };

    schema.to_owned()
}

pub fn numeric_text_optional_one2m() -> String {
    let schema = indoc! {
        r#"model A {
            #id(id, Int, @id)
            float    Float
            int      Int
            decimal      Decimal
            string   String
            b_id Int?
            b    B?   @relation(fields: [b_id], references: [id])
        }

        model B {
            #id(id, Int, @id)
            field  String
            many_a A[]
        }"#
    };

    schema.to_owned()
}
