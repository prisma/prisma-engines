use indoc::indoc;

/// All types Prisma supports on a composite.
/// Allows for simple picking of whatever should be tested from a single model.
pub fn all_composite_types() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            allOptional AllOptional?
            allRequired AllRequired?
            allLists    AllLists?
        }

        enum TestEnum {
            Foo
            Bar
        }

        type AllRequired {
            str   String
            bool  Boolean
            int   Int
            bInt  BigInt
            float Float
            dt    DateTime
            json  Json
            bytes Bytes
            enum  TestEnum
        }

        type AllOptional {
            str   String?
            bool  Boolean?
            int   Int?
            bInt  BigInt?
            float Float?
            dt    DateTime?
            json  Json?
            bytes Bytes?
            enum  TestEnum?
        }

        type AllLists {
            str String[]
            bool  Boolean[]
            int   Int[]
            bInt  BigInt[]
            float Float[]
            dt    DateTime[]
            json  Json[]
            bytes Bytes[]
            enum  TestEnum[]
        }
        "#
    };

    schema.to_owned()
}

/// Full to-one composite test model.
pub fn to_one_composites() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            field String?
            a     A       @map("top_a")
            b     B?
        }

        type A {
            a_1 String @default("a_1 default") @map("a1")
            a_2 Int?
        }

        type B {
            b_field String @default("b_field default")
            c C @map("nested_c")
        }

        type C {
            c_field String @default("c_field default")
            b B?
        }
        "#
    };

    schema.to_owned()
}

/// Full to-many composite test model.
pub fn to_many_composites() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            field String?
            a     A[]       @map("top_a")
            c     C[]       @map("top_c")
        }

        type A {
            a_1 String @default("a_1 default") @map("a1")
            a_2 Int?
            b B[]
        }

        type B {
            b_field String   @default("b_field default")
            a       A[]      @map("nested_a")
        }

        type C {
          c_field String @default("c_field default")
        }
        "#
    };

    schema.to_owned()
}
