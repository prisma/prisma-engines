use crate::common::*;
use datamodel::render_datamodel_to_string;

#[test]
fn constraint_names() {
    let dml = r#"
    //explicit names
    
    model A {
      id   Int    @id("A_primary_key") 
      name String @unique("A_name_unique") 
      a    String 
      b    String
      // c    String @default("Test", map: "A_a_default") //just on sql server
      // B    B[]    @relation("AtoB")
    
      @@unique([a, b], name: "compound", map: "A_unique_index")
      @@index([a], map: "A_index")
    }
    
    model B {
      a   String
      b   String
      // aId Int
      // A   A      @relation("AtoB", fields: [aId], references: [id], map: "B_relation_fkey")
    
      @@id([a, b], name: "ab", map: "B_primary_key")
    }
    
    //no explicit names

    model A2 {
      id   Int    @id 
      name String @unique
      a    String 
      b    String
      // c    String @default("Test")
      // B2    B2[]    @relation("A2toB2")
    
      @@unique([a, b])
      @@index([a])
    }
    
    model B2 {
      a   String
      b   String
      // aId Int
      // A2   A2      @relation("A2toB2", fields: [aId], references: [id])
    
      @@id([a, b], name: "compoundid")
    }
    
    model B3 {
      a   String
      b   String
    
      @@id([a, b])
    }
    "#;

    let res = parse(dml);

    let rendered = render_datamodel_to_string(&res);

    println!("{:#?}", res);

    println!("{}", rendered);

    assert_eq!(true, false);
}
