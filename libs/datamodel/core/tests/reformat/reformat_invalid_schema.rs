use pretty_assertions::assert_eq;

// add validation at the end

#[test]
fn forward_relation_fields_must_be_added() {
    let input = r#"model PostableEntity {
          id String @id
         }
         
         model Post {
            id        String   @id
            postableEntities PostableEntity[]
         }
"#;

    let expected = r#"model PostableEntity {
            id        String @id
            postId    String @relation(fields: postId, references: id)      
            Post      Post?
         }
         
         model Post {
            id                  String   @id
            postableEntities    PostableEntity[]
         }
"#;

    assert_reformat(input, expected);
}

//todo add validation at the end
fn assert_reformat(schema: &str, expected_result: &str) {
    println!("schema: {:?}", schema);
    let result = datamodel::ast::reformat::Reformatter::new(&schema).reformat_to_string();
    println!("result: {}", result);
    assert_eq!(result, expected_result);
}
