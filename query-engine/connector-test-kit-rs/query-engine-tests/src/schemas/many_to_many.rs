use indoc::indoc;

pub fn simple_m2m() -> String {
    let schema = indoc! {
        r#"
        model ModelA {
            #id(id, String, @id)
            #m2m(manyB, ModelB[], id, String)
          }

          model ModelB {
            #id(id, String, @id)
            #m2m(manyA, ModelA[], id, String)
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
            #m2m(categories, Category[], id, Int)
        }

        model Category {
            #id(id, Int, @id)
            name String
            #m2m(posts, Post[], id, Int)
        }
        "#
    };

    schema.to_owned()
}

pub fn posts_on_categories() -> String {
    let schema = indoc! {
        r#"
        model Post {
            #id(id, Int, @id)
            title  String
            categories CategoriesOnPosts[]
          }
          
          model Category {
            #id(id, Int, @id)
            name  String
            posts CategoriesOnPosts[]
          }
          
          model CategoriesOnPosts {
            post       Post     @relation(fields: [postId], references: [id])
            postId     Int 
            category   Category @relation(fields: [categoryId], references: [id])
            categoryId Int 
            tmp Int?
          
            @@id([postId, categoryId])
          }
        "#
    };

    schema.to_owned()
}
