use crate::common::*;
use datamodel::ast::Span;
use datamodel::diagnostics::DatamodelError;
use datamodel::dml::ScalarType;
use datamodel::{render_datamodel_to_string, FieldArity, FieldType, ScalarField};
use pretty_assertions::assert_eq;

#[test]
fn must_add_back_relation_fields_for_given_list_field() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[]
    }

    model Post {
        post_id Int @id
    }
    "#;

    let schema = parse(dml);

    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_relation_field("posts")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&[])
        .assert_arity(&datamodel::dml::FieldArity::List);

    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_relation_field("User")
        .assert_relation_to("User")
        .assert_relation_base_fields(&["userId"])
        .assert_relation_referenced_fields(&["id"])
        .assert_arity(&datamodel::dml::FieldArity::Optional);
    post_model
        .assert_has_scalar_field("userId")
        .assert_base_type(&datamodel::dml::ScalarType::Int);
}

#[test]
fn must_add_back_relation_fields_for_given_singular_field() {
    let dml = r#"
    model User {
        id     Int @id
        postId Int 
        
        post   Post @relation(fields: [postId], references: [post_id]) 
    }

    model Post {
        post_id Int @id
    }
    "#;

    let schema = dbg!(parse(dml));

    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_relation_field("post")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&["post_id"])
        .assert_arity(&datamodel::dml::FieldArity::Required);

    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_relation_field("User")
        .assert_relation_to("User")
        .assert_relation_base_fields(&[])
        .assert_relation_referenced_fields(&[])
        .assert_arity(&datamodel::dml::FieldArity::List);
}

#[test]
fn must_render_generated_back_relation_fields() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[]
    }

    model Post {
        post_id Int @id
    }"#;

    let expected = r#"model User {
  id    Int    @id
  posts Post[]
}

model Post {
  post_id Int   @id
  User    User? @relation(fields: [userId], references: [id])
  userId  Int?
}
"#;

    let schema = parse(dml);

    let rendered = dbg!(render_datamodel_to_string(&schema).unwrap());

    assert_eq!(rendered, expected);
}

#[test]
#[ignore]
fn must_add_referenced_fields_on_the_right_side_for_one_to_one_relations() {
    // the to fields are always added to model with the lower name in lexicographic order
    let dml = r#"
    model User1 {
      id         String @id @default(cuid())
      referenceA User2
    }

    model User2 {
      id         String @id @default(cuid()) 
      referenceB User1
    }

    model User3 {
      id         String @id @default(cuid()) 
      referenceB User4
    }

    model User4 {
      id         String @id @default(cuid())
      referenceA User3
    }
    "#;

    let schema = parse(dml);

    schema
        .assert_has_model("User1")
        .assert_has_relation_field("referenceA")
        .assert_relation_referenced_fields(&["id"]);

    schema
        .assert_has_model("User2")
        .assert_has_relation_field("referenceB")
        .assert_relation_referenced_fields(&[]);

    schema
        .assert_has_model("User3")
        .assert_has_relation_field("referenceB")
        .assert_relation_referenced_fields(&["id"]);

    schema
        .assert_has_model("User4")
        .assert_has_relation_field("referenceA")
        .assert_relation_referenced_fields(&[]);
}

#[test]
#[ignore]
fn must_add_referenced_fields_correctly_for_one_to_one_relations() {
    // Post is lower that User. So the references should be stored in Post.
    let dml = r#"
    model User {
        user_id Int  @id
        post    Post
    }

    model Post {
        post_id Int  @id
        user    User
    }
    "#;

    let schema = dbg!(parse(dml));

    schema
        .assert_has_model("User")
        .assert_has_relation_field("post")
        .assert_relation_referenced_fields(&[]);
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("user")
        .assert_relation_referenced_fields(&["user_id"]);
}

#[test]
fn must_add_referenced_fields_on_both_sides_for_many_to_many_relations() {
    let dml = r#"
    model User {
        user_id Int    @id
        posts   Post[]
    }

    model Post {
        post_id Int    @id
        users   User[]
    }
    "#;

    let schema = parse(dml);

    schema
        .assert_has_model("User")
        .assert_has_relation_field("posts")
        .assert_relation_referenced_fields(&["post_id"]);
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("users")
        .assert_relation_referenced_fields(&["user_id"]);
}

#[test]
#[ignore]
fn must_add_referenced_fields_on_both_sides_for_one_to_many_relations() {
    let dml = r#"
    model User {
        user_id Int    @id
        posts   Post[]
    }

    model Post {
        post_id Int    @id
        user    User
    }
    "#;

    let schema = parse(dml);

    schema
        .assert_has_model("User")
        .assert_has_relation_field("posts")
        .assert_relation_referenced_fields(&[]);
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("user")
        .assert_relation_referenced_fields(&["user_id"]);

    // prove that lexicographic order does not have an influence.
    let dml = r#"
    model User {
        user_id Int    @id
        post    Post
    }

    model Post {
        post_id Int    @id
        users   User[]
    }
    "#;

    let schema = parse(dml);

    schema
        .assert_has_model("User")
        .assert_has_relation_field("post")
        .assert_relation_referenced_fields(&["post_id"]);
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("users")
        .assert_relation_referenced_fields(&[]);
}

#[test]
fn should_not_add_back_relation_fields_for_many_to_many_relations() {
    // Equal name for both fields was a bug trigger.
    let dml = r#"
model Blog {
  id Int @id
  authors Author[]
}

model Author {
  id Int @id
  authors Blog[]
}
    "#;

    let schema = parse(dml);

    let author_model = schema.assert_has_model("Author");
    author_model
        .assert_has_relation_field("authors")
        .assert_relation_to("Blog")
        .assert_relation_name("AuthorToBlog")
        .assert_arity(&datamodel::dml::FieldArity::List);

    author_model.assert_has_scalar_field("id");

    let blog_model = schema.assert_has_model("Blog");
    blog_model
        .assert_has_relation_field("authors")
        .assert_relation_to("Author")
        .assert_relation_name("AuthorToBlog")
        .assert_arity(&datamodel::dml::FieldArity::List);

    blog_model.assert_has_scalar_field("id");

    // Assert nothing else was generated.
    // E.g. no erronous back relations.
    assert_eq!(author_model.fields().count(), 2);
    assert_eq!(blog_model.fields().count(), 2);
}

#[test]
fn should_add_back_relations_for_more_complex_cases() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[]
    }

    model Post {
        post_id Int @id
        comments Comment[]
        categories PostToCategory[]
    }

    model Comment {
        comment_id Int @id
    }

    model Category {
        category_id Int @id
        posts PostToCategory[]
    }

    model PostToCategory {
        id          Int @id
        postId      Int
        categoryId  Int
        
        post     Post     @relation(fields: [postId], references: [post_id])
        category Category @relation(fields: [categoryId], references: [category_id])
        @@map("post_to_category")
    }
    "#;

    let schema = parse(dml);

    // PostToUser

    // Forward
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("User")
        .assert_relation_to("User")
        .assert_relation_referenced_fields(&["id"])
        .assert_relation_name("PostToUser")
        .assert_is_generated(true)
        .assert_arity(&datamodel::dml::FieldArity::Optional);

    // Backward
    schema
        .assert_has_model("User")
        .assert_has_relation_field("posts")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&[])
        .assert_relation_name("PostToUser")
        .assert_is_generated(false)
        .assert_arity(&datamodel::dml::FieldArity::List);

    // Comments

    // Forward
    schema
        .assert_has_model("Comment")
        .assert_has_relation_field("Post")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&["post_id"])
        .assert_relation_name("CommentToPost")
        .assert_is_generated(true)
        .assert_arity(&datamodel::dml::FieldArity::Optional);

    // Backward
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("comments")
        .assert_relation_to("Comment")
        .assert_relation_referenced_fields(&[])
        .assert_relation_name("CommentToPost")
        .assert_is_generated(false)
        .assert_arity(&datamodel::dml::FieldArity::List);

    // CategoryToPostToCategory

    // Backward
    schema
        .assert_has_model("Category")
        .assert_has_relation_field("posts")
        .assert_relation_to("PostToCategory")
        .assert_relation_referenced_fields(&[])
        .assert_relation_name("CategoryToPostToCategory")
        .assert_is_generated(false)
        .assert_arity(&datamodel::dml::FieldArity::List);

    // Forward
    schema
        .assert_has_model("PostToCategory")
        .assert_has_relation_field("category")
        .assert_relation_to("Category")
        .assert_relation_referenced_fields(&["category_id"])
        .assert_relation_name("CategoryToPostToCategory")
        .assert_is_generated(false)
        .assert_arity(&datamodel::dml::FieldArity::Required);

    // PostToPostToCategory

    // Backward
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("categories")
        .assert_relation_to("PostToCategory")
        .assert_relation_referenced_fields(&[])
        .assert_relation_name("PostToPostToCategory")
        .assert_is_generated(false)
        .assert_arity(&datamodel::dml::FieldArity::List);

    // Forward
    schema
        .assert_has_model("PostToCategory")
        .assert_has_relation_field("post")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&["post_id"])
        .assert_relation_name("PostToPostToCategory")
        .assert_is_generated(false)
        .assert_arity(&datamodel::dml::FieldArity::Required);
}

#[test]
#[ignore]
fn should_add_referenced_fields_on_the_correct_side_tie_breaker() {
    let dml = r#"
    model User {
        user_id Int @id
        post Post
    }

    model Post {
        post_id Int @id
        user User
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_relation_field("post")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&[]);

    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to("User")
        .assert_relation_referenced_fields(&["user_id"]);
}

#[test]
#[ignore]
fn should_add_referenced_fields_on_the_correct_side_list() {
    let dml = r#"
    model User {
        id Int @id
        post Post[]
    }

    model Post {
        post_id Int @id
        user User
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_relation_field("post")
        .assert_relation_to("Post")
        .assert_relation_referenced_fields(&[]);

    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to("User")
        .assert_relation_referenced_fields(&["id"]);
}

#[test]
fn should_camel_case_back_relation_field_name() {
    let dml = r#"
    model OhWhatAUser {
        id Int @id
        posts Post[]
    }

    model Post {
        post_id Int @id
    }
    "#;

    let schema = parse(dml);
    schema
        .assert_has_model("Post")
        .assert_has_relation_field("OhWhatAUser")
        .assert_relation_to("OhWhatAUser");
}

#[test]
fn must_add_back_relation_fields_for_self_relations() {
    let dml = r#"
    model Human {
        id    Int @id
        sonId Int?
        
        son   Human? @relation(fields: [sonId], references: [id]) 
    }
    "#;

    let schema = parse(dml);
    let model = schema.assert_has_model("Human");
    model
        .assert_has_relation_field("son")
        .assert_relation_to("Human")
        .assert_arity(&FieldArity::Optional)
        .assert_relation_referenced_fields(&["id"]);

    model
        .assert_has_relation_field("Human")
        .assert_relation_to("Human")
        .assert_arity(&FieldArity::List)
        .assert_relation_referenced_fields(&[]);
}

#[test]
#[ignore]
fn should_add_embed_ids_on_self_relations() {
    let dml = r#"
    model Human {
        id Int @id
        father Human? @relation("paternity")
        son Human? @relation("paternity")
    }
    "#;

    let schema = parse(dml);
    let model = schema.assert_has_model("Human");
    model
        .assert_has_relation_field("son")
        .assert_relation_to("Human")
        .assert_relation_referenced_fields(&[]);

    model
        .assert_has_relation_field("father")
        .assert_relation_to("Human")
        // Fieldname tie breaker.
        .assert_relation_referenced_fields(&["id"]);
}

#[test]
fn should_not_get_confused_with_complicated_self_relations() {
    let dml = r#"
    model Human {
        id        Int  @id
        husbandId Int?
        fatherId  Int?
        parentId  Int?
        
        wife     Human? @relation("Marrige")
        husband  Human? @relation("Marrige", fields: husbandId, references: id)
        
        father   Human? @relation("Paternity", fields: fatherId, references: id)
        son      Human? @relation("Paternity")
        
        children Human[] @relation("Offspring")
        parent   Human? @relation("Offspring", fields: parentId, references: id)
    }
    "#;

    let schema = parse(dml);
    let model = schema.assert_has_model("Human");
    model
        .assert_has_relation_field("son")
        .assert_relation_to("Human")
        .assert_relation_referenced_fields(&[]);

    model
        .assert_has_relation_field("father")
        .assert_relation_to("Human")
        // Fieldname tie breaker.
        .assert_relation_referenced_fields(&["id"]);

    model
        .assert_has_relation_field("wife")
        .assert_relation_to("Human")
        .assert_relation_name("Marrige")
        .assert_relation_referenced_fields(&[]);

    model
        .assert_has_relation_field("husband")
        .assert_relation_to("Human")
        .assert_relation_name("Marrige")
        .assert_relation_referenced_fields(&["id"]);

    model
        .assert_has_relation_field("children")
        .assert_relation_to("Human")
        .assert_relation_name("Offspring")
        .assert_relation_referenced_fields(&[]);

    model
        .assert_has_relation_field("parent")
        .assert_relation_to("Human")
        .assert_relation_name("Offspring")
        .assert_relation_referenced_fields(&["id"]);
}

#[test]
fn must_handle_conflicts_with_existing_fields_if_types_are_compatible() {
    let dml = r#"
    model Blog {
      id    String @id
      posts Post[]
    }
    
    model Post {
      id     String   @id      
      blogId String?
    }
    "#;

    let schema = parse(dml);
    let post = schema.assert_has_model("Post");
    let blog_id_fields: Vec<&ScalarField> = post.scalar_fields().filter(|f| &f.name == "blogId").collect();
    dbg!(&post.fields);
    assert_eq!(blog_id_fields.len(), 1);

    post.assert_has_relation_field("Blog")
        .assert_relation_base_fields(&["blogId"]);
}

#[test]
fn must_handle_conflicts_with_existing_fields_if_types_are_incompatible() {
    let dml = r#"
    model Blog {
      id    String @id
      posts Post[]
    }
    
    model Post {
      id     String   @id      
      blogId Int?     // this is not compatible with Blog.id  
    }
    "#;

    let schema = parse(dml);
    let post = schema.assert_has_model("Post");

    dbg!(&post.fields);

    let underlying_field = post.find_field("blogId_BlogToPost").unwrap();
    assert!(underlying_field.arity().is_optional());
    assert_eq!(underlying_field.field_type(), FieldType::Base(ScalarType::String, None));

    let field = post.assert_has_relation_field("Blog");
    field.assert_relation_base_fields(&["blogId_BlogToPost"]);
}

#[test]
fn must_handle_conflicts_with_existing_fields_if_types_are_incompatible_and_name_generation_breaks_down() {
    let dml = r#"
    model Blog {
      id    String @id
      posts Post[]
    }
    
    model Post {
      id                String   @id      
      blogId            Int?     // this is not compatible with Blog.id
      blogId_BlogToPost Int?     // clashes with the auto generated name
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_model_validation_error(
        "Automatic underlying field generation tried to add the field `blogId_BlogToPost` in model `Post` for the back relation field of `posts` in `Blog`. A field with that name exists already and has an incompatible type for the relation. Please add the back relation manually.",
        "Post",
        Span::new(75,281),
    ));
}
