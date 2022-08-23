use crate::{common::*, with_header, Provider};

#[test]
fn expanded_index_capability_rendering_works() {
    let dm = r#"
        datasource db {
            provider = "mysql"
            url = env("TEST_DATABASE_URL")
        }

        model User {
        id         Int    @id
        firstName  String @unique(sort: Desc, length: 5)
        middleName String @unique(sort: Desc)
        lastName   String @unique(sort: Asc, length: 5)
        generation Int    @unique
        
        @@index([firstName(sort: Desc), middleName(sort: Asc, length: 5), lastName(sort: Desc, length: 5), generation(sort:Asc)])
        @@unique([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation(sort:Asc)])
    }
    "#;

    let expected = expect![[r#"
     model User {
       id         Int    @id
       firstName  String @unique(length: 5, sort: Desc)
       middleName String @unique(sort: Desc)
       lastName   String @unique(length: 5)
       generation Int    @unique
     
       @@unique([firstName(sort: Desc), middleName(length: 5), lastName(length: 5, sort: Desc), generation])
       @@index([firstName(sort: Desc), middleName(length: 5), lastName(length: 5, sort: Desc), generation])
     }
    "#]];

    let dml = parse(dm);
    let configuration = datamodel::parse_configuration(dm).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&configuration));
    expected.assert_eq(&rendered)
}

#[test]
fn expanded_id_capability_rendering_works_for_mysql() {
    let dm = with_header(
        r#"
      model User {
        id         String @id(length: 15)
      }
      
      model User2 {
        firstName  String 
        lastName   String 
        
        @@id([firstName, lastName(length: 5)])
      }
    "#,
        Provider::Mysql,
        &[],
    );

    let expected = expect![[r#"
      model User {
        id String @id(length: 15)
      }
      
      model User2 {
        firstName String
        lastName  String

        @@id([firstName, lastName(length: 5)])
      }
    "#]];

    let dml = parse(&dm);
    let configuration = datamodel::parse_configuration(&dm).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&configuration));
    expected.assert_eq(&rendered)
}

#[test]
fn expanded_id_capability_rendering_works_for_sqlserver() {
    let dm = with_header(
        r#"
      model User {
        id         String @id(sort: Asc)
      }
      
      model User2 {
        firstName  String 
        lastName   String 
        
        @@id([firstName(sort: Asc), lastName(sort: Desc)])
      }
    "#,
        Provider::SqlServer,
        &[],
    );

    let expected = expect![[r#"
      model User {
        id String @id
      }
      
      model User2 {
        firstName String
        lastName  String

        @@id([firstName, lastName(sort: Desc)])
      }
    "#]];

    let dml = parse(&dm);
    let configuration = parse_configuration(&dm);
    let rendered = datamodel::render_datamodel_to_string(&dml, Some(&configuration));
    expected.assert_eq(&rendered)
}
