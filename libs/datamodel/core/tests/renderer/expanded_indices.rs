use expect_test::expect;

#[test]
fn expanded_index_capability_rendering_works() {
    let dm = r#"
        model User {
        id         Int    @id
        firstName  String @unique(sort:Desc, length: 5)
        middleName String @unique(sort:Desc)
        lastName   String @unique(length: 5)
        generation Int    @unique
        
        @@index([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
        @@unique([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
    }
    "#;

    let expected = expect![[r#"
        model User {
        id         Int    @id
        firstName  String @unique(sort:Desc, length: 5)
        middleName String @unique(sort:Desc)
        lastName   String @unique(length: 5)
        generation Int    @unique
        
        @@index([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
        @@unique([firstName(sort: Desc), middleName(length: 5), lastName(sort: Desc, length: 5), generation])
    }
    "#]];

    let dml = datamodel::parse_datamodel(dm).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml, None);
    expected.assert_eq(&rendered)
}

#[test]
fn expanded_id_capability_rendering_works() {
    let dm = r#"
      model User {
        id         String @id(length: 15)
      }
      
      model User2 {
        firstName  String 
        lastName   String 
        
        @@id([firstName, lastName( length: 5)])
      }
    "#;

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

    let dml = datamodel::parse_datamodel(dm).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml, None);
    expected.assert_eq(&rendered)
}
