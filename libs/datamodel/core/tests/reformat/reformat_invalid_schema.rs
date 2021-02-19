use datamodel::parse_datamodel;
use indoc::indoc;
use pretty_assertions::assert_eq;

// scalar above corresponding relationfield?

#[test]
fn forward_relation_fields_must_be_added() {
    let input = indoc! {r#"
        model PostableEntity {
            id String @id
        }
         
        model Post {
            id        String   @id
            postableEntities PostableEntity[]
        }
"#};

    let expected = indoc! {r#"
         model PostableEntity {
           id     String  @id
           Post   Post?   @relation(fields: [postId], references: [id])
           postId String?
         }
         
         model Post {
           id               String           @id
           postableEntities PostableEntity[]
         }
         
"#};

    assert_reformat(input, expected);
}

#[test]
fn must_add_back_relation_fields_for_given_list_field() {
    let input = indoc! {r#"
    model User {
        id Int @id
        posts Post[]
    }

    model Post {
        post_id Int @id
    }
    "#};

    let expected = indoc! {r#"
    model User {
      id    Int    @id
      posts Post[]
    }

    model Post {
      post_id Int   @id
      User    User? @relation(fields: [userId], references: [id])
      userId  Int?
    }
    "#};

    assert_reformat(input, expected);
}

#[test]
fn must_add_back_relation_fields_for_given_singular_field() {
    let input = indoc! {r#"
    model User {
        id     Int @id
        postId Int     
        post   Post @relation(fields: [postId], references: [post_id]) 
    }

    model Post {
        post_id Int @id
    }
    "#};

    let expected = indoc! {r#"
    model User {
      id     Int  @id
      postId Int
      post   Post @relation(fields: [postId], references: [post_id])
    }
    
    model Post {
      post_id Int    @id
      User    User[]
    }
    "#};

    assert_reformat(input, expected);
}

#[test]
fn must_add_back_relation_fields_for_self_relations() {
    let input = indoc! {r#"
    model Human {
        id    Int @id
        sonId Int?
        son   Human? @relation(fields: [sonId], references: [id]) 
    }
    "#};

    let expected = indoc! {r#"
    model Human {
      id    Int     @id
      sonId Int?
      son   Human?  @relation(fields: [sonId], references: [id])
      Human Human[] @relation("HumanToHuman")
    }
    "#};

    assert_reformat(input, expected);
}

fn assert_reformat(schema: &str, expected_result: &str) {
    println!("schema: {:?}", schema);
    //make sure expecation is valid
    parse_datamodel(expected_result).unwrap();

    //reformat input
    let result = datamodel::ast::reformat::Reformatter::new(&schema).reformat_to_string();
    //make sure reformatted input is valid
    parse_datamodel(&result).unwrap();

    println!("result: {}", result);
    assert_eq!(result, expected_result);
}
