use indoc::indoc;

pub fn simple_m2m() -> String {
    let schema = indoc! {
        r#"
        model ModelA {
            #id(id, String, @id)
            #m2m(manyB, ModelB[], String)
          }

          model ModelB {
            #id(id, String, @id)
            #m2m(manyA, ModelA[], String)
          }
        "#
    };

    schema.to_owned()
}

/// User <-m---n-> posts
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
