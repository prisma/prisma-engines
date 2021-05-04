use crate::common::*;

#[test]
fn constraint_names() {
    let dml = r#"
    //explicit names
    
    model A {
      id   Int    @id("A_pkey") 
      // name String @unique("A_name_key") 
      // a    String @default("Test", map: "A_a_dflt") //just on sql server
      // b    String
      // B    B[]    @relation("AtoB")
    
      // @@unique([a, b], name: "compound", map: "A_a_b_key")
      // @@index([a], map: "A_a_idx")
    }
    
    model B {
      a   String
      b   String
      // aId Int
      // A   A      @relation("AtoB", fields: [aId], references: [id], map: "B_aId_fkey")
    
      @@id([a, b], name: "ab", map: "B_pkey")
    }
    
    //no explicit names

    model A2 {
      id   Int    @id 
      // name String @unique
      // a    String @default("Test")
      // b    String
      // B2    B2[]    @relation("A2toB2")
    
      // @@unique([a, b])
      // @@index([a])
    }
    
    model B2 {
      a   String
      b   String
      // aId Int
      // A2   A2      @relation("A2toB2", fields: [aId], references: [id])
    
      @@id([a, b])
    }
    "#;

    let res = parse(dml);

    println!("{:#?}", res);

    assert_eq!(true, false);
}
