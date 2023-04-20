use crate::common::*;

#[test]
fn allow_ignore_on_valid_model() {
    let dml = r#"
    model ModelId {
      a String @id

      @@ignore
    }

    model ModelUnique {
      a String @unique

      @@ignore
    }

    model ModelCompoundId {
      a     String
      int  Int

      @@id([a, int])
      @@ignore
    }

    model ModelCompoundUnique {
      a     String
      int  Int

      @@unique([a, int])
      @@ignore
    }
    "#;

    let datamodel = psl::parse_schema(dml).unwrap();
    datamodel.assert_has_model("ModelId").assert_ignored(true);
    datamodel.assert_has_model("ModelUnique").assert_ignored(true);
    datamodel.assert_has_model("ModelCompoundId").assert_ignored(true);
    datamodel.assert_has_model("ModelCompoundUnique").assert_ignored(true);
}

#[test]
fn allow_ignore_on_invalid_models() {
    let dml = r#"
    model ModelNoFields {

      @@ignore
    }

    model ModelNoId {
      text String

      @@ignore
    }

    model ModelOptionalId {
      text String? @id

      @@ignore
    }

    model ModelUnsupportedId {
      text Unsupported("something") @id

      @@ignore
    }

    model ModelCompoundUnsupportedId {
      text Unsupported("something")
      int  Int

      @@id([text, int])
      @@ignore
    }
    "#;

    let datamodel = psl::parse_schema(dml).unwrap();
    datamodel.assert_has_model("ModelNoFields").assert_ignored(true);
    datamodel.assert_has_model("ModelNoId").assert_ignored(true);
    datamodel.assert_has_model("ModelOptionalId").assert_ignored(true);
    datamodel.assert_has_model("ModelUnsupportedId").assert_ignored(true);
    datamodel
        .assert_has_model("ModelCompoundUnsupportedId")
        .assert_ignored(true);
}

#[test]
fn allow_ignore_on_valid_models_in_relations() {
    let dml = r#"
    model ModelValidA {
      id Int @id
      b  Int
      rel_b  ModelValidB @relation(fields:b, references: id)

      @@ignore
    }

    model ModelValidB {
      id Int @id
      rel_a  ModelValidA[] @ignore
    }

    model ModelValidC {
      id Int @id
      d  Int
      rel_d  ModelValidD @relation(fields:d, references: id) @ignore
    }

    model ModelValidD {
      id Int @id
      rel_c  ModelValidC[]

      @@ignore
    }
    "#;

    let datamodel = psl::parse_schema(dml).unwrap();
    datamodel
        .assert_has_model("ModelValidA")
        .assert_ignored(true)
        .assert_has_relation_field("rel_b")
        .assert_ignored(false);
    datamodel
        .assert_has_model("ModelValidB")
        .assert_ignored(false)
        .assert_has_relation_field("rel_a")
        .assert_ignored(true);
    datamodel
        .assert_has_model("ModelValidC")
        .assert_ignored(false)
        .assert_has_relation_field("rel_d")
        .assert_ignored(true);
    datamodel
        .assert_has_model("ModelValidD")
        .assert_ignored(true)
        .assert_has_relation_field("rel_c")
        .assert_ignored(false);
}

#[test]
fn allow_ignore_on_invalid_models_in_relations() {
    let dml = r#"
    model ModelInvalidA {
      id Unsupported("something") @id
      b  Int
      rel_b  ModelValidB @relation(fields:b, references: id)

      @@ignore
    }

    model ModelValidB {
      id Int @id
      rel_a  ModelInvalidA[] @ignore
    }

    model ModelInvalidC {
      id Unsupported("something") @id
      d  Int
      rel_d  ModelValidD @relation(fields:d, references: id)

      @@ignore
    }

    model ModelValidD {
      id Int @id
      rel_c  ModelInvalidC[]

      @@ignore
    }
    "#;

    let datamodel = parse_schema(dml);
    datamodel
        .assert_has_model("ModelInvalidA")
        .assert_ignored(true)
        .assert_has_relation_field("rel_b")
        .assert_ignored(false);
    datamodel
        .assert_has_model("ModelValidB")
        .assert_ignored(false)
        .assert_has_relation_field("rel_a")
        .assert_ignored(true);
    datamodel
        .assert_has_model("ModelInvalidC")
        .assert_ignored(true)
        .assert_has_relation_field("rel_d")
        .assert_ignored(false);
    datamodel
        .assert_has_model("ModelValidD")
        .assert_ignored(true)
        .assert_has_relation_field("rel_c")
        .assert_ignored(false);
}

#[test]
fn allow_ignore_on_scalar_fields() {
    let dml = r#"
    datasource test {
        provider = "postgresql"
        url = "postgresql://"
    }

    model ModelA {
      id Int   @id
      b  Int   @ignore
      c  Int   @unique @ignore  // required + unique                           => client api adjustment?
      e  Int   @unique @default(1) @ignore // unique + required + default      => client api adjustment?
      f  Int   @default(1) @ignore  //                                         => client api adjustment?
      g  Int?  @unique @ignore
      h  Int?  @unique @default(1) @ignore //                                  => client api adjustment?
      i  Int[] @unique @ignore      //                                         => client api adjustment?
    }
    "#;

    let datamodel = parse_schema(dml);
    datamodel
        .assert_has_model("ModelA")
        .assert_has_scalar_field("b")
        .assert_ignored(true);
}

#[test]
fn allow_ignore_on_scalar_fields_that_are_used() {
    let dml = r#"
    model ModelA {
      id Int   @unique
      a  Int
      b  Int   @ignore

      @@id([a,b])
      @@unique([a,b])
      @@index([b])
    }
    "#;

    let datamodel = parse_schema(dml);
    datamodel
        .assert_has_model("ModelA")
        .assert_has_scalar_field("b")
        .assert_ignored(true);
}

#[test]
fn allow_ignore_on_relation_fields_on_valid_models() {
    let dml = r#"
    model ModelValidA {
      id Int @id
      b  Int
      rel_b  ModelValidB @relation(fields:b, references: id)
    }

    model ModelValidB {
      id Int @id
      rel_a  ModelValidA[] @ignore
    }

    model ModelValidC {
      id Int @id
      d  Int
      rel_d  ModelValidD @relation(fields:d, references: id) @ignore
    }

    model ModelValidD {
      id Int @id
      rel_c  ModelValidC[]
    }

    model ModelValidE {
      id Int @id
      e  Int
      rel_f  ModelValidF @relation(fields:e, references: id) @ignore
    }

    model ModelValidF {
      id Int @id
      rel_e  ModelValidE[] @ignore
    }
    "#;

    let datamodel = parse_schema(dml);
    datamodel
        .assert_has_model("ModelValidA")
        .assert_has_relation_field("rel_b")
        .assert_ignored(false);
    datamodel
        .assert_has_model("ModelValidB")
        .assert_has_relation_field("rel_a")
        .assert_ignored(true);
    datamodel
        .assert_has_model("ModelValidC")
        .assert_has_relation_field("rel_d")
        .assert_ignored(true);
    datamodel
        .assert_has_model("ModelValidD")
        .assert_has_relation_field("rel_c")
        .assert_ignored(false);
    datamodel
        .assert_has_model("ModelValidE")
        .assert_has_relation_field("rel_f")
        .assert_ignored(true);
    datamodel
        .assert_has_model("ModelValidF")
        .assert_has_relation_field("rel_e")
        .assert_ignored(true);
}
