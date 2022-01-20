use expect_test::expect;

#[test]
fn many_to_many_relation_fields_with_referential_actions() {
    let schema = r#"
datasource db {
  provider = "sqlite"
  url      = "file:./dev.db"
}

model Track {
  id        String     @id
  title     String
  playlists Playlist[] @relation(onDelete: Restrict, onUpdate: Restrict)
}

model Playlist {
  id     String  @id
  name   String
  tracks Track[] @relation(onDelete: Restrict, onUpdate: Restrict)
}
    "#;

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: Referential actions on implicit many-to-many relations are not supported[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  name   String
        [1;94m16 | [0m  tracks Track[] @relation(onDelete: [1;91mRestrict[0m, onUpdate: Restrict)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Referential actions on implicit many-to-many relations are not supported[0m
          [1;94m-->[0m  [4mschema.prisma:16[0m
        [1;94m   | [0m
        [1;94m15 | [0m  name   String
        [1;94m16 | [0m  tracks Track[] @relation(onDelete: Restrict, onUpdate: [1;91mRestrict[0m)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Referential actions on implicit many-to-many relations are not supported[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  title     String
        [1;94m10 | [0m  playlists Playlist[] @relation(onDelete: [1;91mRestrict[0m, onUpdate: Restrict)
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating: Referential actions on implicit many-to-many relations are not supported[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  title     String
        [1;94m10 | [0m  playlists Playlist[] @relation(onDelete: Restrict, onUpdate: [1;91mRestrict[0m)
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&datamodel::parse_schema(schema).map(drop).unwrap_err());
}
