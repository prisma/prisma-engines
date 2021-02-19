use crate::common::*;
use datamodel::ast::Span;
use datamodel::diagnostics::DatamodelError;
use datamodel::{dml, ScalarType};

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
        .assert_field_count(3)
        .assert_has_relation_field("posts")
        .assert_relation_to("Post")
        .assert_arity(&dml::FieldArity::List)
        .assert_relation_name("PostToUser");
    user_model
        .assert_has_relation_field("more_posts")
        .assert_relation_to("Post")
        .assert_arity(&dml::FieldArity::List)
        .assert_relation_name("more_posts");

    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_field_count(6)
        .assert_has_scalar_field("text")
        .assert_base_type(&ScalarType::String);
    post_model
        .assert_has_relation_field("user")
        .assert_relation_to("User")
        .assert_arity(&dml::FieldArity::Required)
        .assert_relation_name("PostToUser");
    post_model
        .assert_has_relation_field("posting_user")
        .assert_relation_to("User")
        .assert_arity(&dml::FieldArity::Required)
        .assert_relation_name("more_posts");
}

#[test]
fn allow_complicated_self_relations() {
    let dml = r#"
    model User {
        id     Int  @id
        sonId  Int?
        wifeId Int?
        
        son     User? @relation(name: "offspring", fields: sonId, references: id)
        father  User? @relation(name: "offspring")
        
        husband User? @relation(name: "spouse")
        wife    User? @relation(name: "spouse", fields: wifeId, references: id)
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model.assert_has_relation_field("son").assert_relation_to("User");
    user_model
        .assert_has_relation_field("father")
        .assert_relation_to("User");
    user_model
        .assert_has_relation_field("husband")
        .assert_relation_to("User");
    user_model.assert_has_relation_field("wife").assert_relation_to("User");
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
//todo decide if and where to move these

//reformat implicit relations test files

//todo this talked about adding backrelation fields but was adding forward field + scalarfield
#[test]
fn must_generate_forward_relation_fields_for_named_relation_fields() {
    //reject, hint to prisma format, add scalar field and relation field, validate again
    let dml = r#"
    model Todo {
        id Int @id
        assignees User[] @relation(name: "AssignedTodos")
    }

    model User {
        id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_field_validation_error("The relation field `assignees` on Model `Todo` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.", "Todo", "assignees",Span::new(45, 95)),
    );
}

// todo this is also accepted and adds a postId scalar field under the hood on PostableEntity
// is almost the exact same case as the one above (minus the relationname), but reported as a bug and also understood by harshit as such
#[test]
fn issue4850() {
    //reject, hint to prisma format, add scalar field and relation field, validate again
    let dml = r#"
         model PostableEntity {
          id String @id
         }
         
         model Post {
            id        String   @id
            postableEntities PostableEntity[]
         }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_field_validation_error("The relation field `postableEntities` on Model `Post` is missing an opposite relation field on the model `PostableEntity`. Either run `prisma format` or add it manually.", "Post", "postableEntities",Span::new(147, 181)),
    );
}

//todo I think this should be fine and just add the @relation and relationname to the backrelation field
// but this interprets the dm as containing two relations.
#[test]
fn issue4822() {
    //reject, ask to name custom_Post relation
    let dml = r#"
         model Post {
           id          Int    @id 
           user_id     Int    @unique
           custom_User User   @relation("CustomName", fields: [user_id], references: [id])
         }
                 
         model User {
           id          Int    @id 
           custom_Post Post?  
         }
    "#;

    let errors = parse_error(dml);
    errors.assert_are(
        &[DatamodelError::new_field_validation_error("The relation field `custom_User` on Model `Post` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.", "Post", "custom_User",Span::new(107, 187)), 
            DatamodelError::new_field_validation_error("The relation field `custom_Post` on Model `User` is missing an opposite relation field on the model `Post`. Either run `prisma format` or add it manually.", "User", "custom_Post",Span::new(284, 304))
        ],
    );
}

//todo this is also accepted and adds a organizationId scalar field under the hood
#[test]
fn issue5216() {
    //reject,
    let dml = r#"
         model user {
            id             String        @id 
            email          String        @unique
            organization   organization? @relation(references: [id])
         }
         
         model organization {
            id        String   @id
            users     user[]
         }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_attribute_validation_error("The relation field `organization` on Model `user` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.", "relation",Span::new(130, 187)),
    );
}

//todo this is also accepted but will under the hood point the createdBy relationfield to the same userId scalar
// as the user relationfield
// duplicate of 5540
// comment by matt:
// We don't want to remove the formatting feature that adds @relation and foreign key, this is a beloved feature.
// We want the validator to ensure that @relation always exists and links to a valid field.
// If the formatter is unable to correctly add @relation because of an ambiguity (e.g. user & createdBy), it shouldn't try. The validator will just tell you that you're missing @relation and need to add them in by hand to resolve the issue.
#[test]
fn issue5069() {
    // reject
    let dml = r#"
         model Code {
          id          String        @id
          createdById String?
          createdBy   User?
                   
          userId      String?
          user        User?         @relation("code", fields: [userId], references: [id])
        
        }
        
        model User {
          id         String         @id
          codes      Code[]         @relation("code")
        }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_field_validation_error("The relation field `createdBy` on Model `Code` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.", "Code", "createdBy",Span::new(103, 121)),
    );
}
