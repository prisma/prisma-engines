use super::common::*;
use datamodel::dml::*;

#[test]
fn scalar_fields_map_to_a_single_datasource_field() {
    let dml = r#"
    model Model {
        id String @id
    }
    "#;

    let datamodel = parse(dml);

    assert_eq!(
        datamodel
            .assert_has_model("Model")
            .assert_has_field("id")
            .assert_has_one_datasource_field(),
        &DataSourceField {
            name: "id".to_owned(),
            arity: FieldArity::Required,
            field_type: ScalarType::String,
            default_value: None
        }
    );
}

#[test]
fn relation_fields_only_have_a_datasource_field_when_they_are_not_virtual() {
    let dml = r#"
    model Blog {
        id Int @id
        posts Post[]
    }
    model Post {
        id String @id
        blog Blog @relation(references: [id])
    }
    "#;

    let datamodel = parse(dml);

    assert_eq!(
        datamodel
            .assert_has_model("Post")
            .assert_has_field("blog")
            .assert_has_one_datasource_field(),
        &DataSourceField {
            name: "blog".to_owned(),
            arity: FieldArity::Required,
            field_type: ScalarType::Int,
            default_value: None
        }
    );

    datamodel
        .assert_has_model("Blog")
        .assert_has_field("posts")
        .assert_has_no_datasource_fields();
}

#[test]
fn relation_fields_only_have_multiple_datasource_field_when_they_are_compound() {
    let dml = r#"
    model Blog {
        id Int @id
        author User @relation(references: [firstName, lastName])
    }
    model User {
        id Int @id
        firstName String
        lastName  Int
        blogs Blog[]
        @@unique([firstName, lastName])
    }
    "#;

    let datamodel = parse(dml);

    assert_eq!(
        datamodel
            .assert_has_model("Blog")
            .assert_has_field("author")
            .assert_has_multiple_datasource_fields(),
        vec![
            &DataSourceField {
                name: "author_firstName".to_owned(),
                arity: FieldArity::Required,
                field_type: ScalarType::String,
                default_value: None
            },
            &DataSourceField {
                name: "author_lastName".to_owned(),
                arity: FieldArity::Required,
                field_type: ScalarType::Int,
                default_value: None
            },
        ]
    );

    datamodel
        .assert_has_model("User")
        .assert_has_field("blogs")
        .assert_has_no_datasource_fields();
}

#[test]
fn must_respect_custom_db_names() {
    let dml = r#"
    model Blog {
        id Int @id @map("blog_id") 
        author User @relation(references: [firstName, lastName]) @map(["author_fn", "author_ln"])
    }
    model User {
        id Int @id
        firstName String
        lastName  Int
        @@unique([firstName, lastName])
    }
    "#;

    let datamodel = parse(dml);

    assert_eq!(
        datamodel
            .assert_has_model("Blog")
            .assert_has_field("id")
            .assert_has_one_datasource_field(),
        &DataSourceField {
            name: "blog_id".to_owned(),
            arity: FieldArity::Required,
            field_type: ScalarType::Int,
            default_value: None
        }
    );

    assert_eq!(
        datamodel
            .assert_has_model("Blog")
            .assert_has_field("author")
            .assert_has_multiple_datasource_fields(),
        vec![
            &DataSourceField {
                name: "author_fn".to_owned(),
                arity: FieldArity::Required,
                field_type: ScalarType::String,
                default_value: None
            },
            &DataSourceField {
                name: "author_ln".to_owned(),
                arity: FieldArity::Required,
                field_type: ScalarType::Int,
                default_value: None
            },
        ]
    );
}
