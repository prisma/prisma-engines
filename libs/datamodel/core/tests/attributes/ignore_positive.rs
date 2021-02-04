use crate::common::*;

// @@ignore
// can be on any model in a relation
// holding the fk -> backrelation ignored
// having relation pointing to it -> rf ignored

//@ignore
// can be on any relation field
// should we then ignore also the opposite side???
// can be on any scalar field
// if it is in a relation
// not in a relation
// unique
// optional
// required
// not null
// has a default
// index
// id without other unique -> invalid model needs to be ignored as well
// id with other unique
// compound

#[test]
fn allow_ignored_on_valid_model() {
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

    let datamodel = parse(dml);
    datamodel.assert_has_model("ModelId").assert_is_ignored();
    datamodel.assert_has_model("ModelUnique").assert_is_ignored();
    datamodel.assert_has_model("ModelCompoundId").assert_is_ignored();
    datamodel.assert_has_model("ModelCompoundUnique").assert_is_ignored();
}

#[test]
fn allow_ignored_on_invalid_models() {
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

    let datamodel = parse(dml);
    datamodel.assert_has_model("ModelNoFields").assert_is_ignored();
    datamodel.assert_has_model("ModelNoId").assert_is_ignored();
    datamodel.assert_has_model("ModelOptionalId").assert_is_ignored();
    datamodel.assert_has_model("ModelUnsupportedId").assert_is_ignored();
    datamodel
        .assert_has_model("ModelModelCompoundUnsupportedId")
        .assert_is_ignored();
}
