use crate::common::*;
use datamodel::*;

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

    let dml = parse(dm);
    let rendered = render_datamodel_to_string(&dml, None);
    expected.assert_eq(&rendered)
}
