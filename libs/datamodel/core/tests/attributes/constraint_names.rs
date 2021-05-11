use crate::common::*;
use datamodel::render_datamodel_to_string;

#[test]
fn constraint_names() {
    let dml = r#"
    //explicit different dbnames
    
    model A {
      id   Int    @id("CustomDBId") 
      name String @unique("CustomDBUnique") 
    }
    
    model B {
      a   String
      b   String
   
      @@index([a], map: "CustomDBIndex")
      @@unique([a, b], name: "clientUnique", map: "CustomCompoundDBUnique")
      @@id([a, b], name: "clientId", map: "CustomCompoundDBId")
    }
    
    //explicit same dbnames

    model A2 {
      id   Int    @id("A2_id_pkey") 
      name String @unique(A2_name_key)
    }
    
    model B2 {
      a   String
      b   String
   
      @@index([a], map: "B2_a_idx")
      @@unique([a, b], name: "clientUnique", map: "B2_a_b_key")
      @@id([a, b], name: "clientId", map: "B2_a_b_pkey")
    }
    
    //only explicit different dbnames
    
    model A3 {
      id   Int    @id("CustomDBId2") 
      name String @unique("CustomDBUnique2") 
    }
    
    model B3 {
      a   String
      b   String
   
      @@index([a], map: "CustomCompoundDBIndex2")
      @@unique([a, b], map: "CustomCompoundDBUnique2")
      @@id([a, b], map: "CustomCompoundDBId2")
    }
    
    //no db names

    model A4 {
      id   Int    @id 
      name String @unique
    }
    
    model B4 {
      a   String
      b   String
   
      @@index([a])
      @@unique([a, b], name: "clientUnique")
      @@id([a, b], name: "clientId")
    }
    
    //no names

    model A5 {
      id   Int    @id 
      name String @unique
    }
   
    model B5 {
      a   String
      b   String
   
      @@index([a])
      @@unique([a, b])
      @@id([a, b])
    }
    
    //backwards compatibility
    model B6 {
      a   String @id
      b   String
   
      @@index([a], name: "shouldChangeToMap")
    }
    
    "#;

    let res = parse(dml);

    let rendered = render_datamodel_to_string(&res);

    println!("{:#?}", res);

    println!("{}", rendered);

    assert_eq!(true, false);
}
