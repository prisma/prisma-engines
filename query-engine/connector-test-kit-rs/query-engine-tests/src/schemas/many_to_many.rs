use indoc::indoc;

/// User <-1---m-> posts
pub fn posts_categories() -> String {
    let schema = indoc! {
        r#"
        model Post {
            #id(id, Int, @id)
            title   String
            content String @default("Wip")
            #m2m(categories, Category[], Int)
        }

        model Category {
            #id(id, Int, @id)
            name String
            #m2m(posts, Post[], Int)
        }
        "#
    };

    schema.to_owned()
}
