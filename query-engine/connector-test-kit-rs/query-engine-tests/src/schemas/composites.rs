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
            b   B @map("nested_b")
        }

        type B {
            b_field String @default("b_field default")
            c C @map("nested_c")
        }

        type C {
            c_field String @default("c_field default")
            c_opt   String?
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
            to_many_as CompositeA[] @map("top_a")
            to_one_b   CompositeB?
        }

        type CompositeA {
            a_1          String       @default("a_1 default") @map("a1")
            a_2          Int?
            a_to_one_b   CompositeB?
            a_to_many_bs CompositeB[]
        }

        type CompositeB {
            b_field      Int?         @default(10)
            b_to_one_c   CompositeC?
            b_to_many_cs CompositeC[]
        }

        type CompositeC {
          c_field Int @default(10)

          // Mostly here to test ordering multiple nested selection sets.
          c_to_many_as CompositeA[]
        }
        "#
    };

    schema.to_owned()
}

/// Composites and relations mixed.
pub fn mixed_composites() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            field        String?
            to_one_com   CompositeA?  @map("to_one_composite")
            to_many_com  CompositeB[] @map("to_many_composite")

            to_one_rel_id Int? @unique
            to_one_rel    RelatedModel? @relation(name: "ToOne", fields: [to_one_rel_id], references: [id])

            #m2m(to_many_rel, RelatedModel[], id, Int, ToMany)
        }

        type CompositeA {
            a_1              String       @default("a_1 default") @map("a1")
            a_2              Int?
            a_to_other_com   CompositeC?
            other_composites CompositeB[]
            scalar_list      String[]
        }

        type CompositeB {
            b_field       String       @default("b_field default")
            to_other_com  CompositeC?  @map("nested_c")
            to_other_coms CompositeC[]
            scalar_list   String[]
        }

        type CompositeC {
          c_field     String @default("c_field default")
          scalar_list String[]
        }

        model RelatedModel {
            #id(id, Int, @id)

            to_one_com   CompositeA?  @map("to_one_composite")
            to_many_com  CompositeB[] @map("to_many_composite")

            test_model TestModel? @relation(name: "ToOne")
            #m2m(many_test_model, TestModel[], id, Int, ToMany)
        }

        "#
    };

    schema.to_owned()
}
