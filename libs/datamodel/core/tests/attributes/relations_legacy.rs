use crate::common::*;
use datamodel::dml;
use datamodel::{ast::Span, diagnostics::DatamodelError};

// Ported from
// https://github.com/prisma/prisma/blob/master/server/servers/deploy/src/test/scala/com/prisma/deploy/migration/validation/RelationAttributeSpec.scala

//todo move to the other relation test files
#[test]
fn settings_must_be_deteced() {
    let dml = r#"
    model Todo {
      id       Int  @id
      parentId Int?
      
      child_todos Todo[] @relation("MyRelation")
      parent_todo Todo? @relation("MyRelation", fields: parentId, references: id)
    }
    "#;

    let schema = parse(dml);

    let todo_model = schema.assert_has_model("Todo");
    todo_model
        .assert_has_relation_field("parent_todo")
        .assert_relation_to("Todo")
        .assert_relation_referenced_fields(&["id"])
        .assert_arity(&dml::FieldArity::Optional);
    // TODO: bring `onDelete` back once `prisma migrate` is a thing
    //        .assert_relation_delete_strategy(dml::OnDeleteStrategy::Cascade);
}

#[test]
fn fail_if_ambigous_relation_fields_do_not_specify_a_name() {
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
