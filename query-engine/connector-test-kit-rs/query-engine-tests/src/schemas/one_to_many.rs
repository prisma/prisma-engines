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
