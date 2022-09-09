use crate::common::*;

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

    let rendered = rerender(dm);
    expected.assert_eq(&rendered)
}
