use crate::common::*;
use datamodel::render_datamodel_to_with_preview_flags;
use indoc::indoc;

#[test]
fn constraint_names() {
    let input = indoc! {r#"
        datasource test {
          provider = "postgres"
          url = "posgresql://..."
        }

        /// explicit different dbnames
        model A {
          id   Int    @id(map: "CustomDBId")
          name String @unique(map: "CustomDBUnique")
          b_a  String
          b_b  String
          B    B      @relation(fields: [b_a, b_b], references: [a, b], map: "CustomDBFK")
        }

        model B {
          a String
          b String
          A A?

          @@id([a, b], name: "clientId", map: "CustomCompoundDBId")
          @@unique([a, b], name: "clientUnique", map: "CustomCompoundDBUnique")
          @@index([a], map: "CustomDBIndex")
        }

        /// explicit same dbnames
        model A2 {
          id   Int    @id(map: "A2_pkey")
          name String @unique(map: "A2_name_key")
        }

        model B2 {
          a String
          b String

          @@index([a], map: "B2_a_idx")
          @@unique([a, b], name: "clientUnique", map: "B2_a_b_key")
          @@id([a, b], name: "clientId", map: "B2_pkey")
        }

        /// only explicit different dbnames
        model A3 {
          id   Int    @id(map: "CustomDBId2")
          name String @unique(map: "CustomDBUnique2")
        }

        model B3 {
          a String
          b String

          @@index([a], map: "CustomCompoundDBIndex2")
          @@unique([a, b], map: "CustomCompoundDBUnique2")
          @@id([a, b], map: "CustomCompoundDBId2")
        }

        /// no db names
        model A4 {
          id   Int    @id
          name String @unique
        }

        model B4 {
          a String
          b String

          @@index([a])
          @@unique([a, b], name: "clientUnique")
          @@id([a, b], name: "clientId")
        }

        /// no names
        model A5 {
          id   Int    @id
          name String @unique
        }

        model B5 {
          a String
          b String

          @@index([a])
          @@unique([a, b])
          @@id([a, b])
        }

        /// backwards compatibility
        model B6 {
          a String @id
          b String

          @@index([a], name: "shouldChangeToMap")
        }
     "#};

    let expected = expect![[r#"
        /// explicit different dbnames
        model A {
          id   Int    @id(map: "CustomDBId")
          name String @unique(map: "CustomDBUnique")
          b_a  String
          b_b  String
          B    B      @relation(fields: [b_a, b_b], references: [a, b], map: "CustomDBFK")
        
          @@unique([b_a, b_b])
        }

        model B {
          a String
          b String
          A A?

          @@id([a, b], name: "clientId", map: "CustomCompoundDBId")
          @@unique([a, b], name: "clientUnique", map: "CustomCompoundDBUnique")
          @@index([a], map: "CustomDBIndex")
        }

        /// explicit same dbnames
        model A2 {
          id   Int    @id
          name String @unique
        }

        model B2 {
          a String
          b String

          @@id([a, b], name: "clientId")
          @@unique([a, b], name: "clientUnique")
          @@index([a])
        }

        /// only explicit different dbnames
        model A3 {
          id   Int    @id(map: "CustomDBId2")
          name String @unique(map: "CustomDBUnique2")
        }

        model B3 {
          a String
          b String

          @@id([a, b], map: "CustomCompoundDBId2")
          @@unique([a, b], map: "CustomCompoundDBUnique2")
          @@index([a], map: "CustomCompoundDBIndex2")
        }

        /// no db names
        model A4 {
          id   Int    @id
          name String @unique
        }

        model B4 {
          a String
          b String

          @@id([a, b], name: "clientId")
          @@unique([a, b], name: "clientUnique")
          @@index([a])
        }

        /// no names
        model A5 {
          id   Int    @id
          name String @unique
        }

        model B5 {
          a String
          b String

          @@id([a, b])
          @@unique([a, b])
          @@index([a])
        }

        /// backwards compatibility
        model B6 {
          a String @id
          b String

          @@index([a], map: "shouldChangeToMap")
        }
    "#]];

    let datamodel = parse(input);
    let preview_features = parse_configuration(input).preview_features();
    let mut rendered = String::new();

    render_datamodel_to_with_preview_flags(
        &mut rendered,
        &datamodel,
        parse_configuration(input).datasources.first(),
        preview_features,
    );

    //todo can't be exactly the same since explicit default names will be suppressed when rerendering
    // the expected result after parsing and rendering is not exactly the same as the input.
    // One case where a difference occurs is if you explicitly write a constraint name that happens
    // to match the generated default. That one will not be rendered back. Also if you use the name
    // property in @@index it will be rendered back as map instead.
    expected.assert_eq(&rendered)
}
