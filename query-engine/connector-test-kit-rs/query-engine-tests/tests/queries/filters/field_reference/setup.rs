use query_engine_tests::*;

/// Test model containing all possible Prisma scalar types, nullable.
/// Each type is duplicated to ease comparing by reference.
/// Excludes capability-dependent types (e.g. JSON, Decimal).
pub fn common_types() -> String {
    let schema = indoc! {
        "model TestModel {
          #id(id, Int, @id)
          string  String?
          string2 String?
          int     Int?
          int2    Int?
          bInt    BigInt?
          bInt2   BigInt?
          float   Float?
          float2  Float?
          bytes   Bytes?
          bytes2  Bytes?
          bool    Boolean?
          bool2   Boolean?
          dt      DateTime?
          dt2     DateTime?
      }"
    };

    schema.to_owned()
}

/// Creates test data used by filter tests using the `common_types` schema.
pub async fn test_data_common_types(runner: &Runner) -> TestResult<()> {
    runner
        .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 1,
                string: "abc",
                string2: "abc",
                int: 1,
                int2: 1,
                bInt: 1,
                bInt2: 1,
                float: 1.5,
                float2: 1.5,
                bytes: "AQID",
                bytes2: "AQID",
                bool: false,
                bool2: false,
                dt: "1900-10-10T01:10:10.001Z",
                dt2: "1900-10-10T01:10:10.001Z",
            }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"
          mutation { createOneTestModel(data: {
              id: 2,
              string: "abc",
              string2: "bcd",
              int: 1,
              int2: 2,
              bInt: 1,
              bInt2: 2,
              float: 1.5,
              float2: 2.4,
              bytes: "AQID",
              bytes2: "AQIDBA==",
              bool: false,
              bool2: true,
              dt: "1900-10-10T01:10:10.001Z",
              dt2: "1901-10-10T01:10:10.001Z",
          }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
        .await?
        .assert_success();

    Ok(())
}

/// Test model containing all possible Prisma scalar types, nullable.
/// Each type is duplicated to ease comparing by reference.
/// Excludes capability-dependent types (e.g. JSON, Decimal).
pub fn common_mixed_types() -> String {
    let schema = indoc! {
        "model TestModel {
          #id(id, Int, @id)
          string  String?
          string2 String[]
          int     Int?
          int2    Int[]
          bInt    BigInt?
          bInt2   BigInt[]
          float   Float?
          float2  Float[]
          bytes   Bytes?
          bytes2  Bytes[]
          bool    Boolean?
          bool2   Boolean[]
          dt      DateTime?
          dt2     DateTime[]
      }"
    };

    schema.to_owned()
}

/// Creates test data used by filter tests using the `common_mixed_types` schema.
pub async fn test_data_common_mixed_types(runner: &Runner) -> TestResult<()> {
    runner
        .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 1,
                string: "a",
                string2: ["a"],
                int: 1,
                int2: [1],
                bInt: 1,
                bInt2: [1],
                float: 1.5,
                float2: [1.5],
                bytes: "AQID",
                bytes2: ["AQID"],
                bool: false,
                bool2: [false],
                dt: "1900-10-10T01:10:10.001Z",
                dt2: ["1900-10-10T01:10:10.001Z"],
            }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"
          mutation { createOneTestModel(data: {
              id: 2,
              string: "a",
              string2: ["b"],
              int: 1,
              int2: [2],
              bInt: 1,
              bInt2: [2],
              float: 1.5,
              float2: [2.4],
              bytes: "AQID",
              bytes2: ["AQIDBA=="],
              bool: false,
              bool2: [true],
              dt: "1900-10-10T01:10:10.001Z",
              dt2: ["1901-10-10T01:10:10.001Z"],
          }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
        .await?
        .assert_success();

    Ok(())
}

/// Test model containing all possible Prisma scalar types, nullable.
/// Each type is duplicated to ease comparing by reference.
/// Excludes capability-dependent types (e.g. JSON, Decimal).
pub fn common_list_types() -> String {
    let schema = indoc! {
        "model TestModel {
        #id(id, Int, @id)
        string  String[]
        string2 String[]
        int     Int[]
        int2    Int[]
        bInt    BigInt[]
        bInt2   BigInt[]
        float   Float[]
        float2  Float[]
        bytes   Bytes[]
        bytes2  Bytes[]
        bool    Boolean[]
        bool2   Boolean[]
        dt      DateTime[]
        dt2     DateTime[]
    }"
    };

    schema.to_owned()
}

/// Creates test data used by filter tests using the `common_nullable_types` schema.
pub async fn test_data_list_common(runner: &Runner) -> TestResult<()> {
    runner
        .query(indoc! { r#"
        mutation { createOneTestModel(data: {
            id: 1,
            string: ["a", "b"],
            string2: ["a", "b"],
            int: [1, 2],
            int2: [1, 2],
            bInt: [1, 2],
            bInt2: [2, 3],
            float: [1.5, 2.4],
            float2: [1.5, 2.4],
            bytes: ["AQID", "AQIDBA=="],
            bytes2: ["AQID", "AQIDBA=="],
            bool: [false, true],
            bool2: [false, true],
            dt: ["1900-10-10T01:10:10.001Z", "1901-10-10T01:10:10.001Z"],
            dt2: ["1900-10-10T01:10:10.001Z", "1901-10-10T01:10:10.001Z"],
        }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"
        mutation { createOneTestModel(data: {
            id: 2,
            string: ["a", "b"],
            string2: ["b", "c"],
            int: [1, 2],
            int2: [2, 3],
            bInt: [1, 2],
            bInt2: [2, 3],
            float: [1.5, 2.4],
            float2: [2.4, 3.7],
            bytes: ["AQID", "AQIDBA=="],
            bytes2: ["AQIDBA==", "AQIDBAU="],
            bool: [false, true],
            bool2: [true, true],
            dt: ["1900-10-10T01:10:10.001Z", "1901-10-10T01:10:10.001Z"],
            dt2: ["1901-10-10T01:10:10.001Z", "1901-11-10T01:10:10.001Z"],
        }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
        .await?
        .assert_success();

    Ok(())
}

/// Test model containing a mix of composite object & list.
pub fn mixed_composite_types() -> String {
    let schema = indoc! {
        "model TestModel {
            #id(id, Int, @id)
            comp Composite?
            comp2 Composite?
            comp_list Composite[]
            comp_list2 Composite[]
         }

         type Composite {
            string  String
            string2 String
         }
        "
    };

    schema.to_owned()
}

/// Creates test data used by filter tests using the `composite_types` schema.
pub async fn test_data_mixed_composite(runner: &Runner) -> TestResult<()> {
    runner
        .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 1,
                comp: { string: "a", string2: "a" },
                comp_list: [{ string: "a", string2: "a" }, { string: "a", string2: "a" }]
            }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"
            mutation { createOneTestModel(data: {
                id: 2,
                comp: { string: "a", string2: "b" },
                comp_list: [{ string: "a", string2: "b" }, { string: "c", string2: "d" }]
            }) { id }}"# })
        .await?
        .assert_success();

    runner
        .query(indoc! { r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"# })
        .await?
        .assert_success();

    Ok(())
}
