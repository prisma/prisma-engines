use expect_test::expect;

#[test]
fn enum_rendering_works() {
    let dm = r#"
        enum Nat {
          Zero
          Suc

              @@map("naturalNumber")
        }
    "#;

    let expected = expect![[r#"
        enum Nat {
          Zero
          Suc

          @@map("naturalNumber")
        }
    "#]];

    let dml = datamodel::parse_datamodel(dm).unwrap().subject;
    let rendered = datamodel::render_datamodel_to_string(&dml, None);
    expected.assert_eq(&rendered)
}
