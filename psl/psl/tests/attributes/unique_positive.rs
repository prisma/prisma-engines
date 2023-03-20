use crate::{common::*, with_header, Provider};

#[test]
fn basic_unique_index_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique([firstName,lastName])
    }
    "#;

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["firstName", "lastName"]);
}

#[test]
fn must_succeed_on_model_with_unique_criteria() {
    let dml1 = r#"
    model Model {
        id String @id
    }
    "#;
    psl::parse_schema(dml1).unwrap();

    let dml2 = r#"
    model Model {
        a String
        b String
        @@id([a,b])
    }
    "#;
    psl::parse_schema(dml2).unwrap();

    let dml3 = r#"
    model Model {
        unique String @unique
    }
    "#;
    psl::parse_schema(dml3).unwrap();

    let dml4 = r#"
    model Model {
        a String
        b String
        @@unique([a,b])
    }
    "#;
    psl::parse_schema(dml4).unwrap();
}

#[test]
fn single_field_unique_on_enum_field_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role   @unique
    }

    enum Role {
        Admin
        Member
    }
    "#;

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["role"]);
}

#[test]
fn the_name_argument_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        firstName String
        lastName  String

        @@unique([firstName,lastName], name: "MyIndexName")
    }
    "#;

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["firstName", "lastName"])
        .assert_name("MyIndexName");
}

#[test]
fn multiple_unique_must_work() {
    let dml = r#"
        model User {
            id        Int    @id
            firstName String
            lastName  String

            @@unique([firstName,lastName])
            @@unique([firstName,lastName], name: "MyIndexName", map: "MyIndexName")
        }
    "#;

    let schema = psl::parse_schema(dml).unwrap();
    let user_model = schema.assert_has_model("User");

    user_model.assert_unique_on_fields(&["firstName", "lastName"]);
    user_model.assert_unique_on_fields_and_name(&["firstName", "lastName"], "MyIndexName");
}

#[test]
fn multi_field_unique_on_native_type_fields_fields_must_work() {
    let dml = r#"
    datasource ds {
        provider = "mysql"
        url = "mysql://"
    }

    model User {
        id        Int    @id
        role      Bytes
        role2     Bytes @ds.VarBinary(40)

        @@unique([role2, role])
    }
    "#;

    psl::parse_schema(dml).unwrap();
}

#[test]
fn multi_field_unique_indexes_on_enum_fields_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role

        @@unique([role])
    }

    enum Role {
        Admin
        Member
    }
    "#;

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["role"]);
}

#[test]
fn single_field_unique_indexes_on_enum_fields_must_work() {
    let dml = r#"
    model User {
        id        Int    @id
        role      Role   @unique
    }

    enum Role {
        Admin
        Member
    }
    "#;

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["role"]);
}

#[test]
fn named_multi_field_unique_must_work() {
    let dml = indoc! {r#"
        model User {
          a String
          b Int
          @@unique([a,b], name:"ClientName")
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["a", "b"])
        .assert_name("ClientName");
}

#[test]
fn mapped_multi_field_unique_must_work() {
    let dml = indoc! {r#"
        model User {
          a String
          b Int
          @@unique([a,b], map: "dbname")
        }
    "#};

    psl::parse_schema(with_header(dml, Provider::Postgres, &[]))
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["a", "b"])
        .assert_mapped_name("dbname");
}

#[test]
fn mapped_singular_unique_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String @unique(map: "test")
        }
     
        model Model2 {
          a String @unique(map: "test2")
        }
    "#};

    let schema = psl::parse_schema(with_header(dml, Provider::Postgres, &[])).unwrap();

    schema
        .assert_has_model("Model")
        .assert_unique_on_fields(&["a"])
        .assert_mapped_name("test");

    schema
        .assert_has_model("Model2")
        .assert_unique_on_fields(&["a"])
        .assert_mapped_name("test2");
}

#[test]
fn named_and_mapped_multi_field_unique_must_work() {
    let dml = indoc! {r#"
        model Model {
          a String
          b Int

          @@unique([a,b], name: "compoundId", map:"dbname")
        }
    "#};

    let schema = psl::parse_schema(with_header(dml, Provider::Postgres, &[])).unwrap();

    schema
        .assert_has_model("Model")
        .assert_unique_on_fields(&["a", "b"])
        .assert_mapped_name("dbname")
        .assert_name("compoundId");
}

#[test]
fn mapping_unique_to_a_field_name_should_work() {
    let dml = indoc! {r#"
        model User {
          used           Int
          name           String            
          identification Int

          @@unique([name, identification], name: "usedUnique", map: "used")
        }
    "#};

    psl::parse_schema(dml)
        .unwrap()
        .assert_has_model("User")
        .assert_unique_on_fields(&["name", "identification"])
        .assert_name("usedUnique")
        .assert_mapped_name("used");
}

#[test]
fn duplicate_custom_names_on_different_model_should_work() {
    let dml = indoc! {r#"
        model User {
          name           String            
          identification Int

          @@unique([name, identification], name: "duplicateUnique", map: "onUser")
        }
    
        model Post {
          name           String            
          identification Int

          @@unique([name, identification], name: "duplicateUnique", map: "onPost")
        }
    "#};

    let schema = psl::parse_schema(dml).unwrap();
    let user = schema.assert_has_model("User");
    let post = schema.assert_has_model("Post");

    user.assert_unique_on_fields(&["name", "identification"])
        .assert_name("duplicateUnique");

    post.assert_unique_on_fields(&["name", "identification"])
        .assert_name("duplicateUnique");
}
