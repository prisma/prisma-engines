use crate::common::*;
use datamodel::{common::ScalarType, dml};

#[test]
fn allow_multiple_relations() {
    let dml = r#"
    model User {
        id         Int    @id
        more_posts Post[] @relation(name: "more_posts")
        posts      Post[]
    }

    model Post {
        id            Int    @id
        text          String
        userId        Int
        postingUserId Int
        
        user         User   @relation(fields: userId, references: id)
        posting_user User   @relation(name: "more_posts", fields: postingUserId, references: id)
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_field("posts")
        .assert_relation_to("Post")
        .assert_arity(&dml::FieldArity::List)
        .assert_relation_name("PostToUser");
    user_model
        .assert_has_field("more_posts")
        .assert_relation_to("Post")
        .assert_arity(&dml::FieldArity::List)
        .assert_relation_name("more_posts");

    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_field("text")
        .assert_base_type(&ScalarType::String);
    post_model
        .assert_has_field("user")
        .assert_relation_to("User")
        .assert_arity(&dml::FieldArity::Required)
        .assert_relation_name("PostToUser");
    post_model
        .assert_has_field("posting_user")
        .assert_relation_to("User")
        .assert_arity(&dml::FieldArity::Required)
        .assert_relation_name("more_posts");
}

#[test]
fn allow_complicated_self_relations() {
    let dml = r#"
    model User {
        id     Int @id
        sonId  Int
        wifeId Int
        
        son     User @relation(name: "offspring", fields: sonId, references: id)
        father  User @relation(name: "offspring")
        
        husband User @relation(name: "spouse")
        wife    User @relation(name: "spouse", fields: wifeId, references: id)
    }
    "#;

    let schema = parse(dml);

    let user_model = schema.assert_has_model("User");
    user_model.assert_has_field("son").assert_relation_to("User");
    user_model.assert_has_field("father").assert_relation_to("User");
    user_model.assert_has_field("husband").assert_relation_to("User");
    user_model.assert_has_field("wife").assert_relation_to("User");
}

#[test]
fn allow_unambiguous_self_relations_in_presence_of_unrelated_other_relations() {
    let dml = r#"
        model User {
            id          Int @id
            motherId    Int
            
            subscribers Follower[]
            mother      User @relation(fields: motherId, references: id)
        }

        model Follower {
            id        Int   @id
            following User[]
        }
    "#;

    parse(dml);
}

#[test]
fn must_generate_back_relation_fields_for_named_relation_fields() {
    let dml = r#"
    model Todo {
        id Int @id
        assignees User[] @relation(name: "AssignedTodos")
    }

    model User {
        id Int @id
    }
    "#;

    let datamodel = parse(dml);

    datamodel
        .assert_has_model("User")
        .assert_has_field("todo")
        .assert_relation_name("AssignedTodos")
        .assert_relation_to("Todo");
}
