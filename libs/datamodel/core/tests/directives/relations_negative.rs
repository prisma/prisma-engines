use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};

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
        DatamodelError::new_model_validation_error("Ambiguous relation detected. The fields `posts` and `more_posts` in model `User` both refer to `Post`. Please provide different relation names for them by adding `@relation(<name>).", "User", Span::new(45, 57)),
    );
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
        DatamodelError::new_model_validation_error("Ambiguous relation detected. The fields `author1` and `author2` in model `Post` both refer to `User`. Please provide different relation names for them by adding `@relation(<name>).", "Post", Span::new(114, 126)),
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
            Span::new(45, 81)
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
        Span::new(45, 78),
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
        Span::new(45, 56),
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
        Span::new(57, 67),
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
        Span::new(45, 82),
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
        user User @relation(name: "test")
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_is(DatamodelError::new_model_validation_error(
        "Automatic related field generation would cause a naming conflict. Please add an explicit opposite relation field.",
        "User",
        Span::new(90, 107),
    ));
}

#[test]
#[ignore]
// This case is caught by the requirement that named relations
// need to have an opposite field.
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

    errors.assert_is_at(0, DatamodelError::new_model_validation_error(
        "Automatic opposite related field generation would cause a naming conflict. Please add an explicit opposite relation field.",
        "Todo",
        Span::new(98, 152),
    ));
}
