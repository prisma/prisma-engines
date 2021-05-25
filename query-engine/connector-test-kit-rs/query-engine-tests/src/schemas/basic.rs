use indoc::indoc;

/// Most basic datamodel containing only a model with ID
/// for the most rudimentary testing.
pub fn generic() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            field String?
        }"
    };

    schema.to_owned()
}

/// User model with some basic fields and unique constraints.
pub fn user() -> String {
    let schema = indoc! {
        "model User {
            #id(id, Int, @id)
            first_name String
            last_name  String
            email      String    @unique
            birthday   DateTime?

            @@unique([first_name, last_name])
        }"
    };

    schema.to_owned()
}

/// Test model containing all possible Prisma scalar types, nullable.
/// Excludes capability-dependent types (e.g. JSON).
pub fn common_nullable_types() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            string  String?
            int     Int?
            bInt    BigInt?
            float   Float?
            decimal Decimal?
            bytes   Bytes?
            bool    Boolean?
            dt      DateTime?
        }"
    };

    schema.to_owned()
}

/// Test model containing all common Prisma numeric types.
pub fn common_numeric_types() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            int     Int
            bInt    BigInt
            float   Float
            decimal Decimal
        }"
    };

    schema.to_owned()
}

/// Test model containing all common Prisma numeric and string types.
pub fn common_text_and_numeric_types() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            int     Int
            bInt    BigInt
            float   Float
            decimal Decimal
            string  String
        }"
    };

    schema.to_owned()
}

/// Test model containing all common Prisma numeric and string types, optional.
pub fn common_text_and_numeric_types_optional() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            int     Int?
            bInt    BigInt?
            float   Float?
            decimal Decimal?
            string  String?
        }"
    };

    schema.to_owned()
}

/// Test model containing all possible Prisma scalar types.
/// Excludes capability-dependent types (e.g. JSON).
pub fn common_types() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            string  String
            int     Int
            bInt    BigInt
            float   Float
            decimal Decimal
            bytes   Bytes
            bool    Boolean
            dt      DateTime
        }"
    };

    schema.to_owned()
}

/// Test model containing all possible Prisma list types.
/// Excludes capability-dependent types (e.g. JSON).
pub fn common_list_types() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            string  String[]
            int     Int[]
            bInt    BigInt[]
            float   Float[]
            decimal Decimal[]
            bytes   Bytes[]
            bool    Boolean[]
            dt      DateTime[]
        }"
    };

    schema.to_owned()
}

/// Basic Test model containing a single json field.
pub fn json() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            json Json
        }"
    };

    schema.to_owned()
}

/// Basic Test model containing a single optional json field.
pub fn json_opt() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            json Json?
        }"
    };

    schema.to_owned()
}

pub fn string_combination_unique() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            fieldA String
            fieldB String
            fieldC String
            fieldD String

            @@unique([fieldA, fieldB, fieldC, fieldD])
          }"#
    };

    schema.to_owned()
}

pub fn string_combination() -> String {
    let schema = indoc! {
        r#"model TestModel {
            #id(id, Int, @id)
            fieldA String
            fieldB String
            fieldC String
            fieldD String
          }"#
    };

    schema.to_owned()
}

pub fn autoinc_id() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id, @default(autoincrement()))
        }"
    };

    schema.to_owned()
}
