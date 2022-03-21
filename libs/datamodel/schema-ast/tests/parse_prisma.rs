use schema_ast::parser::parse_schema;
use diagnostics::Diagnostics;

#[test]
fn test_parse() {
    let datamodel_string = r"
        model User {
            name    String?  @sortkey @ahihi
            id      Int      @id @default(autoincrement())
            email   String   @unique @ahihi
        }
    ";

    let mut diagnostics = Diagnostics::new();

    let ast = parse_schema(datamodel_string, &mut diagnostics);
    println!("{:#?}", ast);
    
    assert_eq!(1, 1);
}