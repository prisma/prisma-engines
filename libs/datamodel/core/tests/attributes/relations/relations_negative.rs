use crate::common::*;
use datamodel::{ast::Span, diagnostics::DatamodelError};

#[test]
fn fail_if_ambiguous_relation_fields_do_not_specify_a_name() {
    let dml = r#"
    model Todo {
      id Int @id
      comments Comment[]
      comments2 Comment[]
    }

    model Comment {
      id Int @id
      text String
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is_at(
        0,
        DatamodelError::new_model_validation_error("Ambiguous relation detected. The fields `comments` and `comments2` in model `Todo` both refer to `Comment`. Please provide different relation names for them by adding `@relation(<name>).", "Todo", Span::new(41, 60)),
    );
}

#[test]
fn must_error_when_non_existing_fields_are_used() {
    let dml = r#"
    model User {
        id Int @id
        firstName String
        lastName String
        posts Post[]

        @@unique([firstName, lastName])
    }

    model Post {
        id   Int    @id
        text String
        user User   @relation(fields: [authorFirstName, authorLastName], references: [firstName, lastName])
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(
        DatamodelError::new_validation_error(
            "The argument fields must refer only to existing fields. The following fields do not exist in this model: authorFirstName, authorLastName",
            Span::new(254, 287)
        )
    );
}

#[test]
fn should_fail_on_ambiguous_relations_with_automatic_names_1() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[]
        more_posts Post[]
    }

    model Post {
        post_id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_model_validation_error("Ambiguous relation detected. The fields `posts` and `more_posts` in model `User` both refer to `Post`. Please provide different relation names for them by adding `@relation(<name>).", "User", Span::new(45, 58)),
    );
}

#[test]
fn should_fail_on_colliding_implicit_self_relations() {
    let dml = r#"
    model User {
        id          Int      @id @default(autoincrement())
        name        String?

        husband     User?    @relation("MarriagePartners")
        wife        User     @relation("MarriagePartners")

        teacher     User?    @relation("TeacherStudents")
        students    User[]   @relation("TeacherStudents")
}
"#;

    let errors = parse_error(dml);
    errors.assert_are(&[DatamodelError::new_attribute_validation_error(
        "The relation fields `husband` on Model `User` and `wife` on Model `User` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.",
        "relation",
        Span::new(114, 165),
    ),
        DatamodelError::new_attribute_validation_error(
            "The relation fields `husband` on Model `User` and `wife` on Model `User` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.",
            "relation",
            Span::new(114, 165),
        ),
        DatamodelError::new_attribute_validation_error(
            "The relation fields `wife` on Model `User` and `husband` on Model `User` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.",
            "relation",
            Span::new(173, 224),
        ),
        DatamodelError::new_attribute_validation_error(
            "The relation fields `wife` on Model `User` and `husband` on Model `User` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.",
            "relation",
            Span::new(173, 224),
        ),
        DatamodelError::new_attribute_validation_error(
            "The relation field `teacher` on Model `User` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.",
            "relation",
            Span::new(233, 283),
        ),
        DatamodelError::new_attribute_validation_error(
            "The relation field `teacher` on Model `User` must specify the `references` argument in the @relation attribute.",
            "relation",
            Span::new(233, 283),
        )]);
}

#[test]
fn should_fail_on_ambiguous_relations_with_automatic_names_2() {
    // test case based on: https://github.com/prisma/prisma2/issues/976
    let dml = r#"
    model User {
        id Int @id
        posts Post[]
    }

    model Post {
        post_id Int @id
        author1 User
        author2 User
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_model_validation_error("Ambiguous relation detected. The fields `author1` and `author2` in model `Post` both refer to `User`. Please provide different relation names for them by adding `@relation(<name>).", "Post", Span::new(114, 127)),
    );
}

#[test]
fn should_fail_on_ambiguous_relations_with_manual_names_1() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[] @relation(name: "test")
        more_posts Post[] @relation(name: "test")
    }

    model Post {
        post_id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(
        DatamodelError::new_model_validation_error(
            "Wrongly named relation detected. The fields `posts` and `more_posts` in model `User` both use the same relation name. Please provide different relation names for them through `@relation(<name>).",
            "User",
            Span::new(45, 82)
        ),
    );
}

#[test]
fn should_fail_on_ambiguous_relations_with_manual_names_2() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[] @relation(name: "a")
        more_posts Post[] @relation(name: "b")
        some_posts Post[]
        even_more_posts Post[] @relation(name: "a")
    }

    model Post {
        post_id Int @id
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Wrongly named relation detected. The fields `posts` and `even_more_posts` in model `User` both use the same relation name. Please provide different relation names for them through `@relation(<name>).",
        "User",
        Span::new(45, 79),
    ));
}

#[test]
fn should_fail_on_ambiguous_self_relation() {
    let dml = r#"
    model User {
        id Int @id
        father User
        son User
        mother User
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Unnamed self relation detected. The fields `father`, `son` and `mother` in model `User` have no relation name. Please provide a relation name for one of them by adding `@relation(<name>).",
        "User",
        Span::new(45, 57),
    ));
}

#[test]
fn should_fail_on_ambiguous_self_relation_with_two_fields() {
    let dml = r#"
        model User {
            id Int @id
            child User
            mother User
        }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Ambiguous self relation detected. The fields `child` and `mother` in model `User` both refer to `User`. If they are part of the same relation add the same relation name for them with `@relation(<name>)`.",
        "User",
        Span::new(57, 68),
    ));
}

#[test]
fn should_fail_on_ambiguous_named_self_relation() {
    let dml = r#"
    model User {
        id Int @id
        father User @relation(name: "family")
        son User @relation(name: "family")
        mother User @relation(name: "family")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Wrongly named self relation detected. The fields `father`, `son` and `mother` in model `User` have the same relation name. At most two relation fields can belong to the same relation and therefore have the same name. Please assign a different relation name to one of them.",
        "User",
        Span::new(45, 83),
    ));
}

#[test]
fn should_fail_on_conflicting_back_relation_field_name() {
    let dml = r#"
    model User {
        id Int @id
        posts Post[] @relation(name: "test")
        more_posts Post[]
    }

    model Post {
        post_id Int @id
        User User @relation(name: "test")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_are(&[DatamodelError::new_field_validation_error(
        "The relation field `more_posts` on Model `User` is missing an opposite relation field on the model `Post`. Either run `prisma format` or add it manually.",
        "User",
        "more_posts",
        Span::new(90, 108),
    ),
        DatamodelError::new_attribute_validation_error(
            "The relation field `User` on Model `Post` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.",
            "relation",
            Span::new(164, 198),
        ),
        DatamodelError::new_attribute_validation_error(
            "The relation field `User` on Model `Post` must specify the `references` argument in the @relation attribute.",
            "relation",
            Span::new(164, 198),
        )]);
}

#[test]

//todo formatter should make an offer
fn should_fail_when_relation_attribute_is_missing_for_one_to_one_relations() {
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

    let errors = parse_error(dml);

    errors.assert_are(&[
        DatamodelError::new_attribute_validation_error(
        "The relation fields `post` on Model `User` and `user` on Model `Post` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.",
        "relation",
        Span::new(51, 64),
    ),
        DatamodelError::new_attribute_validation_error(
            "The relation fields `post` on Model `User` and `user` on Model `Post` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.",
            "relation",
            Span::new(51, 64),
        ),
        DatamodelError::new_attribute_validation_error(
            "The relation fields `user` on Model `Post` and `post` on Model `User` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.",
            "relation",
            Span::new(121, 134),
        )
        ,
        DatamodelError::new_attribute_validation_error(
            "The relation fields `user` on Model `Post` and `post` on Model `User` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.",
            "relation",
            Span::new(121, 134),
        )]);
}

#[test]
fn should_fail_on_conflicting_generated_back_relation_fields() {
    // More specifically, this should not panic.
    let dml = r#"
    model Todo {
        id Int @id
        author Owner @relation(name: "AuthorTodo")
        delegatedTo Owner? @relation(name: "DelegatedToTodo")
    }

    model Owner {
        id Int @id
        todos Todo[]
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is_at(0, DatamodelError::new_field_validation_error(
        "The relation field `author` on Model `Todo` is missing an opposite relation field on the model `Owner`. Either run `prisma format` or add it manually.",
        "Todo",
        "author",
        Span::new(45, 88),
    ));
}

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
        DatamodelError::new_field_validation_error("The relation field `postableEntities` on Model `Post` is missing an opposite relation field on the model `PostableEntity`. Either run `prisma format` or add it manually.", "Post", "postableEntities",Span::new(138, 172)),
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
        &[DatamodelError::new_field_validation_error("The relation field `custom_User` on Model `Post` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.", "Post", "custom_User",Span::new(106, 186)),
            DatamodelError::new_field_validation_error("The relation field `custom_Post` on Model `User` is missing an opposite relation field on the model `Post`. Either run `prisma format` or add it manually.", "User", "custom_Post",Span::new(265, 283))
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
        DatamodelError::new_attribute_validation_error("The relation field `organization` on Model `user` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.", "relation",Span::new(129, 186)),
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

#[test]
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

    let errors = parse_error(dml);
    errors.assert_are(
        &[DatamodelError::new_attribute_validation_error("The relation field `user` on Model `Post` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.", "relation",Span::new(127, 140)),
            DatamodelError::new_attribute_validation_error("The relation field `user` on Model `Post` must specify the `references` argument in the @relation attribute.", "relation",Span::new(127, 140))
        ],
    );

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

    let errors = parse_error(dml);
    errors.assert_are(
        &[DatamodelError::new_attribute_validation_error("The relation field `post` on Model `User` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.", "relation",Span::new(53, 66)),
            DatamodelError::new_attribute_validation_error("The relation field `post` on Model `User` must specify the `references` argument in the @relation attribute.", "relation",Span::new(53, 66))
        ],
    );
}

#[test]
fn should_fail_on_missing_embed_ids_on_self_relations() {
    let dml = r#"
    model Human {
        id Int @id
        father Human? @relation("paternity")
        son Human? @relation("paternity")
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_are(
        &[DatamodelError::new_attribute_validation_error("The relation fields `father` on Model `Human` and `son` on Model `Human` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.", "relation",Span::new(46, 83)),
            DatamodelError::new_attribute_validation_error("The relation fields `father` on Model `Human` and `son` on Model `Human` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.", "relation",Span::new(46, 83)),
            DatamodelError::new_attribute_validation_error("The relation fields `son` on Model `Human` and `father` on Model `Human` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.", "relation",Span::new(91, 125)),
            DatamodelError::new_attribute_validation_error("The relation fields `son` on Model `Human` and `father` on Model `Human` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.", "relation",Span::new(91, 125)),
        ],
    );
}
