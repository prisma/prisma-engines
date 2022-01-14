use indoc::indoc;

/// Full to-one composite test model.
pub fn to_one_composites() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            field String?
            a     A       @map("nested_a")
            b     B?
        }

        type A {
            a_1 String @map("a1")
            a_2 Int?   @map("a2")
        }

        type B {
            b_field String
            c C @map("nested_c")
        }

        type C {
            c_field String
            b B?
        }
        "#
    };

    schema.to_owned()
}

// pub fn to_one_composites() -> String {
//     let schema = indoc! {
//         r#"model TestModel {
//             #id(id, Int, @id)
//             field String?
//             a     A       @map("nested_a")
//             b     B?
//         }

//         type A {
//             a_1 String @default("a_1 default") @map("a1")
//             a_2 Int?   @map("a2")
//         }

//         type B {
//             b_field String @default("b_field default")
//             c C @map("nested_c")
//         }

//         type C {
//             c_field String @default("c_field default")
//             b B?
//         }
//         "#
//     };

//     schema.to_owned()
// }

// defaults
// maps
// native types
