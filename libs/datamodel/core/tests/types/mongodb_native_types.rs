use crate::common::*;

#[test]
fn array_native_type_should_fail() {
    let dml = r#"
        datasource db {
          provider = "mongodb"
          url      = "mongodb://"
        }

        model Blog {
            id     Int   @id @map("_id")
            post_ids String[] @db.Array(ObjectId)
        }
    "#;

    let error = datamodel::parse_schema(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating field 'post_ids': Native type `db.Array` is deprecated. Please use `db.ObjectId` instead.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m            id     Int   @id @map("_id")
        [1;94m 9 | [0m            post_ids String[] @[1;91mdb.Array(ObjectId)[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}
