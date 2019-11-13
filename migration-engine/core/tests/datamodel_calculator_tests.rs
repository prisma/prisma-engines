#![allow(non_snake_case)]
#![allow(unused)]
mod test_harness;

use datamodel::ast::{FieldArity, SchemaAst};
use migration_connector::steps::*;
use migration_core::migration::datamodel_calculator::*;
use migration_core::migration::datamodel_migration_steps_inferrer::*;
use pretty_assertions::{assert_eq, assert_ne};
use test_harness::parse;

// TODO: We could unify the tests for inferrer and calculator.

#[test]
fn add_CreateModel_to_existing_schema() {
    let dm1 = SchemaAst::empty();
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_DeleteModel_to_existing_schema() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = SchemaAst::empty();

    test(dm1, dm2);
}

#[test]
fn add_UpdateModel_to_existing_schema() {
    let dm1 = parse(
        r#"
        model Post {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Post {
            id String @id @default(cuid())

            @@embedded
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_CreateField_to_existing_schema() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Int?
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_CreateField_for_relation_to_existing_schema() {
    let dm1 = parse(
        r#"
        model Blog {
            id String @id @default(cuid())
        }
        model Post {
            id String @id @default(cuid())
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Blog {
            id String @id @default(cuid())
            posts Post[]
        }
        model Post {
            id String @id @default(cuid())
            blog Blog?
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_DeleteField_to_existing_schema() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Int?
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_UpdateField_to_existing_schema() {
    let dm1 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Int?
        }
    "#,
    );
    let dm2 = parse(
        r#"
        model Test {
            id String @id @default(cuid())
            field Boolean @default(false)
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_CreateEnum_to_existing_schema() {
    let dm1 = SchemaAst::empty();
    let dm2 = parse(
        r#"
        enum Test {
            A
            B
        }
    "#,
    );

    test(dm1, dm2);
}

#[test]
fn add_DeleteEnum_to_existing_schema() {
    let dm1 = parse(
        r#"
        enum Test {
            A
            B
        }
    "#,
    );
    let dm2 = SchemaAst::empty();

    test(dm1, dm2);
}

#[should_panic(
    expected = "The model Test already exists in this Datamodel. It is not possible to create it once more."
)]
#[test]
fn creating_a_model_that_already_exists_must_error() {
    let dm = parse(
        r#"
            model Test {
                id Int @id
            }
        "#,
    );

    let steps = &[MigrationStep::CreateModel(CreateModel {
        model: "Test".to_string(),
    })];

    calculate(&dm, steps);
}

#[should_panic(
    expected = "The field id on model Test already exists in this Datamodel. It is not possible to create it once more."
)]
#[test]
fn creating_a_field_that_already_exists_must_error() {
    let dm = parse(
        r#"
            model Test {
                id Int @id
            }
        "#,
    );

    let steps = vec![MigrationStep::CreateField(CreateField {
        model: "Test".to_string(),
        field: "id".to_string(),
        tpe: "Int".to_owned(),
        arity: FieldArity::Required,
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The type Test already exists in this Datamodel. It is not possible to create it once more.")]
#[test]
fn creating_a_custom_type_that_already_exists_must_error() {
    let dm = parse("type Test = Float");

    let steps = &[MigrationStep::CreateCustomType(CreateCustomType {
        custom_type: "Test".to_owned(),
        r#type: "Test".to_string(),
        arity: FieldArity::Required,
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The type Test does not exist in this Datamodel. It is not possible to delete it.")]
#[test]
fn deleting_a_custom_type_that_does_not_exist_must_error() {
    let dm = parse("");

    let steps = &[MigrationStep::DeleteCustomType(DeleteCustomType {
        custom_type: "Test".to_owned(),
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The enum Test already exists in this Datamodel. It is not possible to create it once more.")]
#[test]
fn creating_an_enum_that_already_exists_must_error() {
    let dm = parse(
        r#"
            enum Test {
                A
                B
            }
        "#,
    );

    let steps = vec![MigrationStep::CreateEnum(CreateEnum {
        r#enum: "Test".to_string(),
        values: Vec::new(),
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The model Test does not exist in this Datamodel. It is not possible to delete it.")]
#[test]
fn deleting_a_model_that_does_not_exist_must_error() {
    let dm = SchemaAst::empty();
    let steps = vec![MigrationStep::DeleteModel(DeleteModel {
        model: "Test".to_string(),
    })];

    calculate(&dm, steps);
}

#[should_panic(
    expected = "The model Test does not exist in this Datamodel. It is not possible to delete a field in it."
)]
#[test]
fn deleting_a_field_that_does_not_exist_must_error() {
    let dm = SchemaAst::empty();
    let steps = vec![MigrationStep::DeleteField(DeleteField {
        model: "Test".to_string(),
        field: "id".to_string(),
    })];

    calculate(&dm, steps);
}

#[should_panic(
    expected = "The field my_field on model Test does not exist in this Datamodel. It is not possible to delete it."
)]
#[test]
fn deleting_a_field_that_does_not_exist_2_must_error() {
    let dm = parse(
        r#"
            model Test {
                id Int @id
            }
        "#,
    );
    let steps = vec![MigrationStep::DeleteField(DeleteField {
        model: "Test".to_string(),
        field: "my_field".to_string(),
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The enum Test does not exist in this Datamodel. It is not possible to delete it.")]
#[test]
fn deleting_an_enum_that_does_not_exist_must_error() {
    let dm = SchemaAst::empty();
    let steps = &[MigrationStep::DeleteEnum(DeleteEnum {
        r#enum: "Test".to_string(),
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The model Test does not exist in this Datamodel. It is not possible to update it.")]
#[test]
fn updating_a_model_that_does_not_exist_must_error() {
    let dm = SchemaAst::empty();
    let steps = &[MigrationStep::UpdateModel(UpdateModel {
        model: "Test".to_string(),
        new_name: None,
    })];

    calculate(&dm, steps);
}

#[should_panic(
    expected = "The model Test does not exist in this Datamodel. It is not possible to update a field in it."
)]
#[test]
fn updating_a_field_that_does_not_exist_must_error() {
    let dm = SchemaAst::empty();
    let steps = &[MigrationStep::UpdateField(UpdateField {
        model: "Test".to_string(),
        field: "id".to_string(),
        new_name: None,
        tpe: None,
        arity: None,
    })];

    calculate(&dm, steps);
}

#[should_panic(
    expected = "The field myField on model Test does not exist in this Datamodel. It is not possible to update it."
)]
#[test]
fn updating_a_field_that_does_not_exist_must_error_2() {
    let dm = parse(
        r#"
            model Test {
                id Int @id
            }
        "#,
    );
    let steps = vec![MigrationStep::UpdateField(UpdateField {
        model: "Test".to_string(),
        field: "myField".to_string(),
        new_name: None,
        tpe: None,
        arity: None,
    })];

    calculate(&dm, steps);
}

#[should_panic(expected = "The enum Test does not exist in this Datamodel. It is not possible to update it.")]
#[test]
fn updating_an_enum_that_does_not_exist_must_error() {
    let dm = SchemaAst::empty();
    let steps = vec![MigrationStep::UpdateEnum(UpdateEnum {
        r#enum: "Test".to_string(),
        new_name: None,
        created_values: vec![],
        deleted_values: vec![],
    })];

    calculate(&dm, steps);
}

// This tests use inferrer to create an end-to-end situation.
fn test(dm1: SchemaAst, dm2: SchemaAst) {
    let steps = infer(&dm1, &dm2);
    let result = calculate(&dm1, steps);

    let dm2 = datamodel::lift_ast(&dm2).unwrap();
    let result = datamodel::lift_ast(&result).unwrap();
    assert_eq!(dm2, result);
}

fn calculate(schema: &SchemaAst, steps: impl AsRef<[MigrationStep]>) -> SchemaAst {
    calculate_impl(schema, steps).unwrap()
}

fn calculate_error(schema: &SchemaAst, steps: impl AsRef<[MigrationStep]>) -> String {
    format!("{}", calculate_impl(schema, steps).unwrap_err())
}

fn calculate_impl(schema: &SchemaAst, steps: impl AsRef<[MigrationStep]>) -> Result<SchemaAst, CalculatorError> {
    let calc = DataModelCalculatorImpl {};
    calc.infer(schema, steps.as_ref())
}

fn infer(dm1: &SchemaAst, dm2: &SchemaAst) -> Vec<MigrationStep> {
    let inferrer = DataModelMigrationStepsInferrerImplWrapper {};
    inferrer.infer(dm1, dm2)
}
