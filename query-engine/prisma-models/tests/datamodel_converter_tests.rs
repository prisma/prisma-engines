#![allow(non_snake_case)]
use prisma_models::*;
use prisma_models::{dml::ReferentialAction, *};
use std::collections::BTreeSet;
use std::sync::Arc;

#[test]
fn set_null_is_not_valid_on_mandatory_fields() {
    let datamodel = convert(
        r#"
        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }

        datasource db {
            provider = "sqlite"
            url = "./dev.db"
            relationMode = "foreignKeys"
        }

        model SomeUser {
            id      Int      @id
            ref     Int
            profile Profile?
          
            @@unique([id, ref])
          }
          
        model Profile {
            id     Int       @id
            user   SomeUser? @relation(fields: [user_id, user_ref], references: [id, ref], onUpdate: SetNull, onDelete: SetNull)
            user_id Int?
            user_ref Int
            
            @@unique([user_id, user_ref])
        }
    "#,
    );

    let relations = datamodel.relations();
    assert_eq!(relations.len(), 1);

    let relation = &relations[0];
    assert_eq!(relation.on_update(), ReferentialAction::SetNull);
    assert_eq!(relation.on_delete(), ReferentialAction::SetNull);

    assert_eq!(relation.is_one_to_one(), true);
    assert_eq!(relation.field_a().name, "user");
    assert_eq!(relation.field_b().name, "profile");

    /// Find the first `RelationField` in the first `Model` in the `Relation` that matches the given predicate.
    /// This would be private once moved into a module.
    fn find_referenced_field_a<'a>(
        relation_field_a: &'a RelationFieldRef,
        model_a: &'a ModelRef,
        field_predicate: impl Fn(&&Field) -> bool,
    ) -> Option<&'a Field> {
        let fields_a = &relation_field_a.relation_info.fields;
        let fields_a_as_set: BTreeSet<String> = fields_a.iter().map(|s| s.to_owned()).collect();
        let referenced_fields = model_a.fields().find_many_from_all(&fields_a_as_set);

        match referenced_fields.into_iter().find(field_predicate) {
            Some(field) => Some(&field),
            _ => None,
        }
    }

    /// Find the first `RelationField` in the first `Model` in the `Relation` that matches the given predicate.
    /// This would be public once moved into a module.
    fn find_referenced_field_a_from_relation(
        relation: &std::sync::Arc<Relation>,
        field_predicate: impl Fn(&&Field) -> bool,
    ) -> Option<Field> {
        let relation_field_a = &relation.field_a();
        let model_a = &relation.model_a();

        match find_referenced_field_a(&relation_field_a, &model_a, field_predicate) {
            // we could technically just return the field name here, avoiding a clone.
            // I think it could make sense to keep the clone if we wanted this as a general utility function
            Some(field) => Some(field.clone()),
            _ => None,
        }
    }

    fn validate(relation: &std::sync::Arc<Relation>) {
        let has_referential_action_set_null =
            [relation.on_update(), relation.on_delete()].contains(&ReferentialAction::SetNull);
        if !has_referential_action_set_null {
            return;
        }

        match find_referenced_field_name_a(&relation, |field| field.is_required()) {
            Some(required_field_name) => {
                let action = if relation.on_update() == ReferentialAction::SetNull {
                    "onUpdate"
                } else {
                    "onDelete"
                };
                dbg!(format!(
                    "Field {} is required, but that is incompatible with the referential action {}: {}.",
                    action,
                    required_field_name.name(),
                    ReferentialAction::SetNull,
                ));
            }
            _ => (),
        }
    }

    // you should see the dbg! output:
    //
    // format!("Field {} is required, but that is incompatible with the referential action {}: {}.",
    // action, required_field_name.name(), ReferentialAction :: SetNull,) = "Field onUpdate is required, but that is incompatible with the referential action user_ref: SetNull."
    // test set_null_is_not_valid_on_mandatory_fields ... ok
    validate(&relation);
}

#[test]
fn an_empty_datamodel_must_work() {
    let datamodel = convert("");
    assert!(datamodel.enums.is_empty());
    assert!(datamodel.models().is_empty());
    assert!(datamodel.relations().is_empty());
}

#[test]
fn converting_enums() {
    let datamodel = convert(
        r#"
            model MyModel {
                id Int @id
                field MyEnum
            }

            enum MyEnum {
                A
                B
                C
            }
        "#,
    );
    let expected_values = vec![
        InternalEnumValue {
            name: "A".to_string(),
            database_name: None,
        },
        InternalEnumValue {
            name: "B".to_string(),
            database_name: None,
        },
        InternalEnumValue {
            name: "C".to_string(),
            database_name: None,
        },
    ];
    let enm = datamodel.enums.iter().find(|e| e.name == "MyEnum").unwrap();
    assert_eq!(enm.values, expected_values);

    let field = datamodel.assert_model("MyModel").assert_scalar_field("field");
    assert_eq!(field.type_identifier, TypeIdentifier::Enum("MyEnum".to_string()));
    assert_eq!(
        field.internal_enum,
        Some(InternalEnum {
            name: "MyEnum".to_string(),
            values: expected_values
        })
    );
}

#[test]
fn models_with_only_scalar_fields() {
    let datamodel = convert(
        r#"
            datasource mydb {
                provider = "postgres"
                url = "postgresql://localhost:5432"
            }

            model Test {
                id Int @id @default(autoincrement())
                int Int
                float Float
                boolean Boolean
                dateTime DateTime
                stringOpt String?
                intList Int[]
            }
        "#,
    );

    let model = datamodel.assert_model("Test");

    model
        .assert_scalar_field("id")
        .assert_type_identifier(TypeIdentifier::Int)
        .assert_is_auto_generated_int_id_by_db();

    model
        .assert_scalar_field("int")
        .assert_type_identifier(TypeIdentifier::Int);

    model
        .assert_scalar_field("float")
        .assert_type_identifier(TypeIdentifier::Float);

    model
        .assert_scalar_field("boolean")
        .assert_type_identifier(TypeIdentifier::Boolean);

    model
        .assert_scalar_field("dateTime")
        .assert_type_identifier(TypeIdentifier::DateTime);

    model
        .assert_scalar_field("stringOpt")
        .assert_type_identifier(TypeIdentifier::String)
        .assert_optional();

    model
        .assert_scalar_field("intList")
        .assert_type_identifier(TypeIdentifier::Int)
        .assert_list();
}

#[test]
fn db_names_work() {
    let datamodel = convert(
        r#"
            model Test {
                id String @id @default(cuid())
                field String @map(name:"my_column")
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    let field = model.assert_scalar_field("field");

    assert_eq!(field.db_name(), "my_column")
}

#[test]
fn scalar_lists_work() {
    let datamodel = convert(
        r#"
            datasource pg {
                provider = "postgres"
                url = "postgres://localhost/postgres"
            }

            model Test {
                id String @id @default(cuid())
                intList Int[]
            }
        "#,
    );
    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("intList")
        .assert_type_identifier(TypeIdentifier::Int)
        .assert_list();
}

#[test]
fn unique_works() {
    let datamodel = convert(
        r#"
            model Test {
                id String @id @default(cuid())
                unique String @unique
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("unique")
        .assert_type_identifier(TypeIdentifier::String)
        .assert_unique();
}

#[test]
fn multi_field_unique_with_1_field_must_be_transformed_to_is_unique_on_field() {
    let datamodel = convert(
        r#"
            model Test {
                id     String @id
                a      String
                b      String
                @@unique([a])
                @@unique([a,b])
                @@index([b,a])
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_indexes_length(2)
        .assert_index(&["a", "b"], IndexType::Unique)
        .assert_index(&["b", "a"], IndexType::Normal);
    model
        .assert_scalar_field("a")
        .assert_type_identifier(TypeIdentifier::String)
        .assert_unique();
}

#[test]
fn multi_field_id_with_1_field_must_be_transformed_to_is_id_on_field() {
    let datamodel = convert(
        r#"
            model Test {
                a String

                @@id([a])
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("a")
        .assert_type_identifier(TypeIdentifier::String)
        .assert_is_id();
}

#[test]
fn uuid_fields_must_work() {
    let datamodel = convert(
        r#"
            model Test {
                id String @id @default(uuid())
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("id")
        .assert_type_identifier(TypeIdentifier::String);
}

#[test]
fn cuid_fields_must_work() {
    let datamodel = convert(
        r#"
            model Test {
                id String @id @default(cuid())
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("id")
        .assert_type_identifier(TypeIdentifier::String);
}

#[test]
fn createdAt_works() {
    let datamodel = convert(
        r#"
            model Test {
                id String @id @default(cuid())
                createdAt DateTime @default(now())
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("createdAt")
        .assert_type_identifier(TypeIdentifier::DateTime);
}

#[test]
fn updatedAt_works() {
    let datamodel = convert(
        r#"
            model Test {
                id String @id @default(cuid())
                updatedAt DateTime @updatedAt
            }
        "#,
    );

    let model = datamodel.assert_model("Test");
    model
        .assert_scalar_field("updatedAt")
        .assert_type_identifier(TypeIdentifier::DateTime)
        .assert_updated_at();
}

#[test]
fn explicit_relation_fields() {
    let datamodel = convert(
        r#"
            model Blog {
                id Int @id
                posts Post[]
            }

            model Post {
                id     Int   @id
                blogId Int?  @map("blog_id")
                blog   Blog? @relation(fields: blogId, references: id)
            }
        "#,
    );

    let relation_name = "BlogToPost";
    let blog = datamodel.assert_model("Blog");
    let post = datamodel.assert_model("Post");
    let relation = datamodel.assert_relation(("Blog", "Post"), relation_name);

    blog.assert_relation_field("posts")
        .assert_list()
        .assert_optional()
        .assert_relation_name(relation_name)
        .assert_side(RelationSide::A);

    post.assert_relation_field("blog")
        .assert_optional()
        .assert_relation_name(relation_name)
        .assert_side(RelationSide::B);

    relation
        .assert_name(relation_name)
        .assert_model_a("Blog")
        .assert_model_b("Post")
        .assert_manifestation(RelationLinkManifestation::Inline(InlineRelation {
            in_table_of_model_name: "Post".to_string(),
        }));
}

#[test]
fn many_to_many_relations() {
    let datamodel = convert(
        r#"
            model Post {
                id Int @id
                blogs Blog[]
            }

            model Blog {
                id Int @id
                posts Post[]
            }
        "#,
    );

    let relation_name = "BlogToPost";
    let blog = datamodel.assert_model("Blog");
    let post = datamodel.assert_model("Post");
    let relation = datamodel.assert_relation(("Blog", "Post"), relation_name);

    blog.assert_relation_field("posts")
        .assert_list()
        .assert_optional()
        .assert_relation_name(relation_name)
        .assert_side(RelationSide::A);

    post.assert_relation_field("blogs")
        .assert_list()
        .assert_optional()
        .assert_relation_name(relation_name)
        .assert_side(RelationSide::B);

    relation
        .assert_name(relation_name)
        .assert_model_a("Blog")
        .assert_model_b("Post")
        .assert_manifestation(RelationLinkManifestation::RelationTable(RelationTable {
            table: format!("_{}", relation_name),
            model_a_column: "A".to_string(),
            model_b_column: "B".to_string(),
        }));
}

#[test]
fn explicit_relation_names() {
    let datamodel = convert(
        r#"
            model Blog {
                id Int @id
                posts Post[] @relation(name: "MyRelationName")
            }

            model Post {
                id     Int  @id
                blogId Int?
                blog   Blog? @relation(name: "MyRelationName", fields: blogId, references: id)
            }
        "#,
    );

    let blog = datamodel.assert_model("Blog");
    let post = datamodel.assert_model("Post");

    let relation_name = "MyRelationName";
    blog.assert_relation_field("posts")
        .assert_list()
        .assert_optional()
        .assert_relation_name(relation_name);
    post.assert_relation_field("blog")
        .assert_optional()
        .assert_relation_name(relation_name);
}

#[test]
#[ignore]
fn self_relations() {
    let datamodel = convert(
        r#"
            model Employee {
                id Int @id
                ReportsTo: Employee?
            }
        "#,
    );

    let employee = datamodel.assert_model("Employee");

    employee
        .assert_relation_field("ReportsTo")
        .assert_relation_name("EmployeeToEmployee");
    // employee.assert_relation_field("employee");
}

#[test]
fn ambiguous_relations() {
    let datamodel = convert(
        r#"
            model Blog {
                id    Int   @id
                post1 Post? @relation(name: "Relation1")
                post2 Post? @relation(name: "Relation2")
            }

            model Post {
                id      Int  @id
                blog1Id Int  @unique
                blog2Id Int  @unique
                blog1   Blog @relation(name: "Relation1", fields: [blog1Id], references: [id])
                blog2   Blog @relation(name: "Relation2", fields: [blog2Id], references: [id])
            }
        "#,
    );

    let blog = datamodel.assert_model("Blog");
    blog.assert_relation_field("post1").assert_relation_name("Relation1");
    blog.assert_relation_field("post2").assert_relation_name("Relation2");

    let post = datamodel.assert_model("Post");
    post.assert_relation_field("blog1").assert_relation_name("Relation1");
    post.assert_relation_field("blog2").assert_relation_name("Relation2");
}

// Regression test
// https://github.com/prisma/prisma/issues/12986
#[test]
fn duplicate_relation_name() {
    let schema = r#"
        model Post {
            id     String @unique
            userId String
            user   User   @relation("a", fields: [userId], references: [id])
          }
          
          model User {
            id       String    @unique
            posts    Post[]    @relation("a")
            comments Comment[] @relation("a")
          }
          
          model Comment {
            id     String @unique
            userId String
            user   User   @relation("a", fields: [userId], references: [id])
          }
          
        "#;

    let dml = psl::parse_schema(schema).unwrap();
    prisma_models::convert(&dml, String::new());
}

#[test]
fn implicit_many_to_many_relation() {
    let datamodel = convert(
        r#"model Post {
                    id         String @id @default(cuid())
                    identifier Int?   @unique
                    related    Post[] @relation(name: "RelatedPosts")
                    parents   Post[] @relation(name: "RelatedPosts")
                  }
                  "#,
    );

    let post = datamodel.assert_model("Post");
    post.assert_relation_field("related");

    post.assert_relation_field("parents");
}

fn convert(datamodel: &str) -> Arc<InternalDataModel> {
    let schema = psl::parse_schema(datamodel).unwrap();
    prisma_models::convert(&schema, "not_important".to_string())
}

trait DatamodelAssertions {
    fn assert_model(&self, name: &str) -> Arc<Model>;
    fn assert_relation(&self, models: (&str, &str), name: &str) -> Arc<Relation>;
}

impl DatamodelAssertions for InternalDataModel {
    fn assert_model(&self, name: &str) -> Arc<Model> {
        self.find_model(name).unwrap()
    }

    fn assert_relation(&self, models: (&str, &str), name: &str) -> Arc<Relation> {
        self.find_relation(models, name).unwrap().upgrade().unwrap()
    }
}

trait ModelAssertions {
    fn assert_indexes_length(&self, len: usize) -> &Self;
    fn assert_index(&self, fields: &[&str], tpe: IndexType) -> &Self;
    fn assert_scalar_field(&self, name: &str) -> Arc<ScalarField>;
    fn assert_relation_field(&self, name: &str) -> Arc<RelationField>;
}

impl ModelAssertions for Model {
    fn assert_indexes_length(&self, len: usize) -> &Self {
        assert_eq!(self.indexes().len(), len);
        self
    }

    fn assert_index(&self, fields: &[&str], tpe: IndexType) -> &Self {
        self.indexes()
            .iter()
            .find(|index| {
                let has_right_type = index.typ == tpe;
                let field_names: Vec<String> = index.fields().iter().map(|f| f.name.clone()).collect();
                let expected_field_names: Vec<String> = fields.iter().map(|f| f.to_string()).collect();
                let is_for_right_fields = field_names == expected_field_names;

                is_for_right_fields && has_right_type
            })
            .unwrap_or_else(|| panic!("Could not find the index for fields {:?} and type {:?}", fields, tpe));
        self
    }

    fn assert_scalar_field(&self, name: &str) -> Arc<ScalarField> {
        self.fields().find_from_scalar(name).unwrap()
    }

    fn assert_relation_field(&self, name: &str) -> Arc<RelationField> {
        self.fields().find_from_relation_fields(name).unwrap()
    }
}

trait FieldAssertions {
    fn assert_type_identifier(&self, ti: TypeIdentifier) -> &Self;
    fn assert_optional(&self) -> &Self;
    fn assert_list(&self) -> &Self;
}

trait ScalarFieldAssertions {
    fn assert_updated_at(&self) -> &Self;
    fn assert_is_auto_generated_int_id_by_db(&self) -> &Self;
    fn assert_is_id(&self) -> &Self;
    fn assert_unique(&self) -> &Self;
}

trait RelationFieldAssertions {
    fn assert_relation_name(&self, name: &str) -> &Self;
    fn assert_side(&self, side: RelationSide) -> &Self;
}

impl FieldAssertions for ScalarField {
    fn assert_type_identifier(&self, ti: TypeIdentifier) -> &Self {
        assert_eq!(self.type_identifier, ti);
        self
    }

    fn assert_optional(&self) -> &Self {
        assert!(!self.is_required());
        self
    }

    fn assert_list(&self) -> &Self {
        assert!(self.is_list());
        self
    }
}

impl ScalarFieldAssertions for ScalarField {
    fn assert_updated_at(&self) -> &Self {
        assert!(self.is_updated_at);
        self
    }

    fn assert_is_auto_generated_int_id_by_db(&self) -> &Self {
        assert!(self.is_auto_generated_int_id);
        self
    }

    fn assert_is_id(&self) -> &Self {
        assert!(self.is_id());
        self
    }

    fn assert_unique(&self) -> &Self {
        assert!(self.unique());
        self
    }
}

impl FieldAssertions for RelationField {
    fn assert_type_identifier(&self, _ti: TypeIdentifier) -> &Self {
        panic!("Can't assert type identifier of relation.")
    }

    fn assert_optional(&self) -> &Self {
        assert!(!self.is_required());
        self
    }

    fn assert_list(&self) -> &Self {
        assert!(self.is_list());
        self
    }
}

impl RelationFieldAssertions for RelationField {
    fn assert_relation_name(&self, name: &str) -> &Self {
        assert_eq!(self.relation_name, name);
        self
    }

    fn assert_side(&self, side: RelationSide) -> &Self {
        assert_eq!(self.relation_side, side);
        self
    }
}

trait RelationAssertions {
    fn assert_name(&self, name: &str) -> &Self;
    fn assert_model_a(&self, name: &str) -> &Self;
    fn assert_model_b(&self, name: &str) -> &Self;
    fn assert_manifestation(&self, mani: RelationLinkManifestation) -> &Self;
}

impl RelationAssertions for Relation {
    fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(self.name, name);
        self
    }
    fn assert_model_a(&self, name: &str) -> &Self {
        assert_eq!(self.model_a().name, name);
        self
    }
    fn assert_model_b(&self, name: &str) -> &Self {
        assert_eq!(self.model_b().name, name);
        self
    }
    fn assert_manifestation(&self, manifestation: RelationLinkManifestation) -> &Self {
        assert_eq!(self.manifestation, manifestation);
        self
    }
}
