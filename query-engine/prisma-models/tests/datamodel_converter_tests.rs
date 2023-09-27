#![allow(non_snake_case)]

use prisma_models::*;
use std::sync::Arc;

#[test]
fn an_empty_datamodel_must_work() {
    let datamodel = convert("");
    assert_eq!(datamodel.schema.db.enums_count(), 0);
    assert_eq!(datamodel.models().count(), 0);
    assert_eq!(datamodel.relations().count(), 0);
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
    let enm = datamodel.find_enum("MyEnum").unwrap();
    assert_eq!(enm.walker().values().len(), 3);
    let model = datamodel.find_model("MyModel").unwrap();
    let field = model.assert_scalar_field("field");
    if let TypeIdentifier::Enum(id) = field.type_identifier() {
        assert_eq!(enm.id, id)
    } else {
        panic!()
    }
}

#[test]
#[should_panic(
    expected = "Indexes can only contain scalar attributes. Please remove all composite types from the argument list of the indexes."
)]
fn converting_composite_types() {
    let datamodel = convert(
        r#"
    datasource db {
      provider = "mongodb"
      url      = "mongodb://localhost:27017/hello"
    }

    model MyModel {
      id        String      @id @default(auto()) @map("_id") @db.ObjectId
      attribute Attribute

      @@unique([attribute], name: "composite_index")
    }

    type Attribute {
      name  String
      value String
      group String
    }
    "#,
    );
    let model = datamodel.assert_model("MyModel");
    let uniq_idx = model.unique_indexes().find(|_| true).unwrap();
    let attr = ScalarField::from((
        model.dm.clone().into(),
        uniq_idx.fields().find(|v| v.name() == "attribute").unwrap(),
    ));
    attr.type_identifier();
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

    convert(schema);
}

fn convert(datamodel: &str) -> InternalDataModel {
    let schema = psl::parse_schema(datamodel).unwrap();
    prisma_models::convert(Arc::new(schema))
}

trait DatamodelAssertions {
    fn assert_model(&self, name: &str) -> Model;
}

impl DatamodelAssertions for InternalDataModel {
    fn assert_model(&self, name: &str) -> Model {
        self.clone().find_model(name).unwrap()
    }
}

trait ModelAssertions {
    fn assert_scalar_field(&self, name: &str) -> ScalarField;
}

impl ModelAssertions for Model {
    fn assert_scalar_field(&self, name: &str) -> ScalarField {
        self.fields().find_from_scalar(name).unwrap()
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

impl FieldAssertions for ScalarField {
    fn assert_type_identifier(&self, ti: TypeIdentifier) -> &Self {
        assert_eq!(self.type_identifier(), ti);
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
        assert!(self.is_updated_at());
        self
    }

    fn assert_is_auto_generated_int_id_by_db(&self) -> &Self {
        assert!(self.is_auto_generated_int_id());
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

trait RelationAssertions {
    fn assert_name(&self, name: &str) -> &Self;
}

impl RelationAssertions for Relation {
    fn assert_name(&self, name: &str) -> &Self {
        assert_eq!(self.name(), name);
        self
    }
}
