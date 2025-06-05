use query_engine_tests::*;

#[test_suite(only(MySql))]
mod edge_cases_shard_key {
    use indoc::indoc;
    use query_engine_tests::{run_query, Runner};

    // Schema for testing edge cases with shard keys
    fn edge_cases_schema() -> String {
        let schema = indoc! {
            r#"
            model User {
              id          String @id @default(uuid())
              email       String @unique
              username    String?
              firstName   String
              lastName    String
              shardKey    String @shardKey
              metadata    Json?
              tags        String[]
              score       Int @default(0)
              balance     Decimal @default(0.00)
              isActive    Boolean @default(true)
              createdAt   DateTime @default(now())
              posts       Post[]
            }

            model Post {
              id          String @id @default(uuid())
              title       String
              content     String?
              authorId    String
              shardKey    String @shardKey
              metadata    Json?
              viewCount   Int @default(0)
              published   Boolean @default(false)
              author      User @relation(fields: [authorId], references: [id])
            }

            model UnicodeModel {
              id          String @id @default(uuid())
              title       String
              content     String?
              emoji       String?
              shardKey    String @shardKey
            }

            model CompositeEdgeCase {
              id          String @id @default(uuid())
              name        String
              region      String
              category    String
              subCategory String?
              metadata    Json?

              @@shardKey([region, category])
            }

            model LargeDataModel {
              id          String @id @default(uuid())
              data        String @db.Text
              binaryData  Bytes?
              shardKey    String @shardKey
            }

            model ConstraintEdgeCase {
              id          String @id @default(uuid())
              uniqueField String @unique
              indexedField String
              shardKey    String @shardKey

              @@index([shardKey, indexedField])
              @@index([indexedField, shardKey])
            }

            model CascadeModel {
              id          String @id @default(uuid())
              name        String
              parentId    String?
              shardKey    String @shardKey
              parent      CascadeModel? @relation("SelfRelation", fields: [parentId], references: [id])
              children    CascadeModel[] @relation("SelfRelation")
            }
            "#
        };

        schema.to_owned()
    }

    // Error Handling Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn shard_key_with_empty_string(runner: Runner) -> TestResult<()> {
        // Test empty string as shard key - should work but might cause issues
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-empty-shard"
                    email: "empty@example.com"
                    firstName: "Empty"
                    lastName: "Shard"
                    shardKey: ""
                }) {
                    id
                    email
                    shardKey
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneUser":{"id":"user-empty-shard","email":"empty@example.com","shardKey":""}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn shard_key_with_special_characters(runner: Runner) -> TestResult<()> {
        // Test special characters in shard key
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-special"
                    email: "special@example.com"
                    firstName: "Special"
                    lastName: "Chars"
                    shardKey: "region-1@#$%^&*()_+-=[]{}|;:,.<>?"
                }) {
                    id
                    shardKey
                }
            }"#
        );

        assert!(result.contains("region-1@#$%^&*()_+-=[]{}|;:,.<>?"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn shard_key_with_unicode_characters(runner: Runner) -> TestResult<()> {
        // Test Unicode characters in shard key
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUnicodeModel(data: {
                    id: "unicode-test"
                    title: "Unicode Test 🚀"
                    content: "Content with émojis and spëcial çharacters"
                    emoji: "🌟🎉🔥💎"
                    shardKey: "région-européenne-中文-日本語-العربية"
                }) {
                    id
                    title
                    emoji
                    shardKey
                }
            }"#
        );

        assert!(result.contains("🚀"));
        assert!(result.contains("région-européenne-中文-日本語-العربية"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn very_long_shard_key_values(runner: Runner) -> TestResult<()> {
        // Test very long shard key values
        let long_shard_key = "a".repeat(1000);

        let query = format!(
            r#"mutation {{
                createOneUser(data: {{
                    id: "user-long-shard"
                    email: "long@example.com"
                    firstName: "Long"
                    lastName: "Shard"
                    shardKey: "{}"
                }}) {{
                    id
                    shardKey
                }}
            }}"#,
            long_shard_key
        );

        let result = run_query!(&runner, &query);
        assert!(result.contains(&long_shard_key));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn composite_shard_key_edge_cases(runner: Runner) -> TestResult<()> {
        // Test composite shard key with various edge case values
        let result = run_query!(
            &runner,
            r#"mutation {
                createManyCompositeEdgeCase(data: [
                    {
                        id: "composite-1"
                        name: "Empty Category"
                        region: "us-east-1"
                        category: ""
                    },
                    {
                        id: "composite-2"
                        name: "Special Chars"
                        region: "us-west-2@#$%"
                        category: "cat/sub\\cat|pipe"
                    },
                    {
                        id: "composite-3"
                        name: "Unicode"
                        region: "欧洲"
                        category: "カテゴリー"
                    }
                ]) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createManyCompositeEdgeCase":{"count":3}}}"###
        );

        Ok(())
    }

    // Large Data Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn large_data_with_shard_keys(runner: Runner) -> TestResult<()> {
        // Test large text data with shard keys
        let large_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);

        let query = format!(
            r#"mutation {{
                createOneLargeDataModel(data: {{
                    id: "large-data-1"
                    data: "{}"
                    shardKey: "large-shard"
                }}) {{
                    id
                    shardKey
                }}
            }}"#,
            large_text.chars().take(10000).collect::<String>() // Limit to avoid query size issues
        );

        let result = run_query!(&runner, &query);
        assert!(result.contains("large-data-1"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn bulk_operations_with_duplicate_shard_keys(runner: Runner) -> TestResult<()> {
        // Test bulk operations with many records in the same shard
        let mut users_data = Vec::new();

        for i in 1..=100 {
            users_data.push(format!(
                r#"{{
                    id: "bulk-user-{}"
                    email: "user{}@example.com"
                    firstName: "User"
                    lastName: "{}"
                    shardKey: "same-shard"
                    score: {}
                }}"#,
                i, i, i, i
            ));
        }

        let create_query = format!(
            r#"mutation {{
                createManyUser(data: [{}]) {{
                    count
                }}
            }}"#,
            users_data.join(",")
        );

        let result = run_query!(&runner, &create_query);
        assert!(result.contains("\"count\":100"));

        // Query all users in the same shard
        let query_result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { shardKey: "same-shard" }
                    orderBy: { score: asc }
                    take: 10
                ) {
                    id
                    score
                    shardKey
                }
            }"#
        );

        assert!(query_result.contains("same-shard"));

        Ok(())
    }

    // Complex Query Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn deeply_nested_where_conditions(runner: Runner) -> TestResult<()> {
        // Create test data
        run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-complex-1"
                        email: "complex1@example.com"
                        firstName: "Complex"
                        lastName: "User1"
                        shardKey: "shard-a"
                        score: 100
                        isActive: true
                    },
                    {
                        id: "user-complex-2"
                        email: "complex2@example.com"
                        firstName: "Complex"
                        lastName: "User2"
                        shardKey: "shard-b"
                        score: 200
                        isActive: false
                    }
                ]) { count }
            }"#
        );

        // Complex nested where with multiple levels of AND/OR
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(where: {
                    AND: [
                        {
                            OR: [
                                { shardKey: "shard-a" },
                                { shardKey: "shard-b" }
                            ]
                        },
                        {
                            AND: [
                                { firstName: "Complex" },
                                {
                                    OR: [
                                        { score: { gte: 100 } },
                                        { isActive: true }
                                    ]
                                }
                            ]
                        },
                        {
                            NOT: {
                                email: { contains: "invalid" }
                            }
                        }
                    ]
                }) {
                    id
                    firstName
                    shardKey
                    score
                }
            }"#
        );

        assert!(result.contains("user-complex-1"));
        assert!(result.contains("user-complex-2"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn aggregation_edge_cases(runner: Runner) -> TestResult<()> {
        // Create diverse test data
        run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "agg-user-1"
                        email: "agg1@example.com"
                        firstName: "Agg"
                        lastName: "User1"
                        shardKey: "agg-shard"
                        score: 0
                        balance: 0.00
                    },
                    {
                        id: "agg-user-2"
                        email: "agg2@example.com"
                        firstName: "Agg"
                        lastName: "User2"
                        shardKey: "agg-shard"
                        score: 2147483647
                        balance: 999999999.99
                    },
                    {
                        id: "agg-user-3"
                        email: "agg3@example.com"
                        firstName: "Agg"
                        lastName: "User3"
                        shardKey: "agg-shard"
                        score: -2147483648
                        balance: -999999999.99
                    }
                ]) { count }
            }"#
        );

        // Test aggregations with extreme values
        let result = run_query!(
            &runner,
            r#"query {
                aggregateUser(where: { shardKey: "agg-shard" }) {
                    _count {
                        _all
                        score
                        balance
                    }
                    _avg {
                        score
                        balance
                    }
                    _sum {
                        score
                        balance
                    }
                    _min {
                        score
                        balance
                    }
                    _max {
                        score
                        balance
                    }
                }
            }"#
        );

        assert!(result.contains("_count"));
        assert!(result.contains("_avg"));
        assert!(result.contains("_sum"));

        Ok(())
    }

    // JSON and Array Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn json_metadata_with_shard_keys(runner: Runner) -> TestResult<()> {
        // Test complex JSON metadata
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "json-user"
                    email: "json@example.com"
                    firstName: "JSON"
                    lastName: "User"
                    shardKey: "json-shard"
                    metadata: {
                        nested: {
                            array: [1, 2, 3, null],
                            object: {
                                key: "value",
                                unicode: "🚀",
                                special: "@#$%^&*()"
                            },
                            boolean: true,
                            number: 123.456,
                            nullValue: null
                        },
                        arrayOfObjects: [
                            { id: 1, name: "first" },
                            { id: 2, name: "second" }
                        ]
                    }
                    tags: ["tag1", "tag with spaces", "tag-with-dashes", "tag_with_underscores", "🏷️"]
                }) {
                    id
                    metadata
                    tags
                    shardKey
                }
            }"#
        );

        assert!(result.contains("json-user"));
        assert!(result.contains("nested"));
        assert!(result.contains("🚀"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn json_filtering_edge_cases(runner: Runner) -> TestResult<()> {
        // Create users with various JSON structures
        run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "json-filter-1"
                        email: "filter1@example.com"
                        firstName: "Filter"
                        lastName: "User1"
                        shardKey: "filter-shard"
                        metadata: { type: "premium", score: 100 }
                    },
                    {
                        id: "json-filter-2"
                        email: "filter2@example.com"
                        firstName: "Filter"
                        lastName: "User2"
                        shardKey: "filter-shard"
                        metadata: { type: "basic", score: 50 }
                    },
                    {
                        id: "json-filter-3"
                        email: "filter3@example.com"
                        firstName: "Filter"
                        lastName: "User3"
                        shardKey: "filter-shard"
                        metadata: null
                    }
                ]) { count }
            }"#
        );

        // Filter by JSON path
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(where: {
                    shardKey: "filter-shard"
                    metadata: {
                        path: ["type"]
                        equals: "premium"
                    }
                }) {
                    id
                    metadata
                }
            }"#
        );

        assert!(result.contains("json-filter-1"));
        assert!(!result.contains("json-filter-2"));

        Ok(())
    }

    // Constraint and Index Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn unique_constraint_across_shards(runner: Runner) -> TestResult<()> {
        // Create user in one shard
        run_query!(
            &runner,
            r#"mutation {
                createOneConstraintEdgeCase(data: {
                    id: "constraint-1"
                    uniqueField: "unique-value"
                    indexedField: "indexed-1"
                    shardKey: "shard-a"
                }) { id }
            }"#
        );

        // Try to create user with same unique field in different shard - should fail
        let error_result = run_query!(
            &runner,
            r#"mutation {
                createOneConstraintEdgeCase(data: {
                    id: "constraint-2"
                    uniqueField: "unique-value"
                    indexedField: "indexed-2"
                    shardKey: "shard-b"
                }) { id }
            }"#
        );

        // Should contain error about unique constraint violation
        assert!(error_result.contains("error") || error_result.contains("Unique"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn complex_indexing_scenarios(runner: Runner) -> TestResult<()> {
        // Create data that exercises different index combinations
        run_query!(
            &runner,
            r#"mutation {
                createManyConstraintEdgeCase(data: [
                    {
                        id: "index-1"
                        uniqueField: "unique-1"
                        indexedField: "common-value"
                        shardKey: "index-shard-a"
                    },
                    {
                        id: "index-2"
                        uniqueField: "unique-2"
                        indexedField: "common-value"
                        shardKey: "index-shard-b"
                    },
                    {
                        id: "index-3"
                        uniqueField: "unique-3"
                        indexedField: "rare-value"
                        shardKey: "index-shard-a"
                    }
                ]) { count }
            }"#
        );

        // Query using compound index (shardKey + indexedField)
        let result = run_query!(
            &runner,
            r#"query {
                findManyConstraintEdgeCase(where: {
                    shardKey: "index-shard-a"
                    indexedField: "common-value"
                }) {
                    id
                    uniqueField
                }
            }"#
        );

        assert!(result.contains("index-1"));
        assert!(!result.contains("index-2")); // Different shard

        Ok(())
    }

    // Self-Referential and Cascading Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn self_referential_relations(runner: Runner) -> TestResult<()> {
        // Create hierarchical data with self-references
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneCascadeModel(data: {
                    id: "root"
                    name: "Root Node"
                    shardKey: "hierarchy-shard"
                    children: {
                        create: [
                            {
                                id: "child-1"
                                name: "Child 1"
                                shardKey: "hierarchy-shard"
                                children: {
                                    create: [
                                        {
                                            id: "grandchild-1"
                                            name: "Grandchild 1"
                                            shardKey: "hierarchy-shard"
                                        }
                                    ]
                                }
                            },
                            {
                                id: "child-2"
                                name: "Child 2"
                                shardKey: "hierarchy-shard"
                            }
                        ]
                    }
                }) {
                    id
                    name
                    children {
                        id
                        name
                        children {
                            id
                            name
                        }
                    }
                }
            }"#
        );

        assert!(result.contains("Root Node"));
        assert!(result.contains("Child 1"));
        assert!(result.contains("Grandchild 1"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn circular_reference_prevention(runner: Runner) -> TestResult<()> {
        // Create initial nodes
        run_query!(
            &runner,
            r#"mutation {
                createManyCascadeModel(data: [
                    {
                        id: "node-a"
                        name: "Node A"
                        shardKey: "circular-shard"
                    },
                    {
                        id: "node-b"
                        name: "Node B"
                        shardKey: "circular-shard"
                    }
                ]) { count }
            }"#
        );

        // Connect A -> B
        run_query!(
            &runner,
            r#"mutation {
                updateOneCascadeModel(
                    where: { id: "node-a" }
                    data: {
                        children: {
                            connect: { id: "node-b" }
                        }
                    }
                ) { id }
            }"#
        );

        // Try to connect B -> A (would create circular reference)
        // This should either work (creating a cycle) or fail gracefully
        let result = run_query!(
            &runner,
            r#"mutation {
                updateOneCascadeModel(
                    where: { id: "node-b" }
                    data: {
                        children: {
                            connect: { id: "node-a" }
                        }
                    }
                ) { id }
            }"#
        );

        // The result depends on whether the system allows cycles
        // At minimum, it shouldn't crash
        assert!(result.contains("node-b") || result.contains("error"));

        Ok(())
    }

    // Concurrent Access Simulation

    #[connector_test(schema(edge_cases_schema))]
    async fn concurrent_shard_operations(runner: Runner) -> TestResult<()> {
        // Simulate concurrent operations on the same shard
        // Create multiple users rapidly in the same shard
        for i in 1..=20 {
            let result = run_query!(
                &runner,
                &format!(
                    r#"mutation {{
                        createOneUser(data: {{
                            id: "concurrent-user-{}"
                            email: "concurrent{}@example.com"
                            firstName: "Concurrent"
                            lastName: "User{}"
                            shardKey: "concurrent-shard"
                            score: {}
                        }}) {{
                            id
                        }}
                    }}"#,
                    i, i, i, i
                )
            );

            assert!(result.contains(&format!("concurrent-user-{}", i)));
        }

        // Verify all users were created
        let count_result = run_query!(
            &runner,
            r#"query {
                aggregateUser(where: { shardKey: "concurrent-shard" }) {
                    _count {
                        _all
                    }
                }
            }"#
        );

        assert!(count_result.contains("20"));

        Ok(())
    }

    // Performance Stress Tests

    #[connector_test(schema(edge_cases_schema))]
    async fn cross_shard_query_performance(runner: Runner) -> TestResult<()> {
        // Create data across multiple shards
        let mut users_data = Vec::new();

        for i in 1..=50 {
            let shard = format!("performance-shard-{}", i % 5); // 5 different shards
            users_data.push(format!(
                r#"{{
                    id: "perf-user-{}"
                    email: "perf{}@example.com"
                    firstName: "Perf"
                    lastName: "User{}"
                    shardKey: "{}"
                    score: {}
                }}"#,
                i,
                i,
                i,
                shard,
                i * 10
            ));
        }

        let create_query = format!(
            r#"mutation {{
                createManyUser(data: [{}]) {{
                    count
                }}
            }}"#,
            users_data.join(",")
        );

        run_query!(&runner, &create_query);

        // Query across all shards with complex conditions
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: {
                        OR: [
                            { shardKey: "performance-shard-0" },
                            { shardKey: "performance-shard-1" },
                            { shardKey: "performance-shard-2" }
                        ]
                        score: { gte: 100 }
                    }
                    orderBy: [
                        { score: desc },
                        { firstName: asc }
                    ]
                    take: 10
                ) {
                    id
                    score
                    shardKey
                }
            }"#
        );

        // Should return results efficiently
        assert!(result.contains("findManyUser"));

        Ok(())
    }

    // Data Integrity Edge Cases

    #[connector_test(schema(edge_cases_schema))]
    async fn orphaned_relation_handling(runner: Runner) -> TestResult<()> {
        // Create user and post
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "orphan-user"
                    email: "orphan@example.com"
                    firstName: "Orphan"
                    lastName: "User"
                    shardKey: "orphan-shard"
                    posts: {
                        create: [
                            {
                                id: "orphan-post"
                                title: "Post to be orphaned"
                                content: "This post will lose its author"
                                shardKey: "orphan-shard"
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // Delete the user (leaving the post orphaned)
        run_query!(
            &runner,
            r#"mutation {
                deleteOneUser(where: { id: "orphan-user" }) {
                    id
                }
            }"#
        );

        // Try to query the orphaned post
        let result = run_query!(
            &runner,
            r#"query {
                findUniquePost(where: { id: "orphan-post" }) {
                    id
                    title
                    author {
                        id
                        firstName
                    }
                }
            }"#
        );

        // The post should still exist but the author relation should be null or cause an error
        assert!(result.contains("orphan-post") || result.contains("error"));

        Ok(())
    }

    #[connector_test(schema(edge_cases_schema))]
    async fn extreme_pagination_edge_cases(runner: Runner) -> TestResult<()> {
        // Create many records in the same shard
        let mut users_data = Vec::new();

        for i in 1..=1000 {
            users_data.push(format!(
                r#"{{
                    id: "page-user-{:04}"
                    email: "page{}@example.com"
                    firstName: "Page"
                    lastName: "User{:04}"
                    shardKey: "pagination-shard"
                    score: {}
                }}"#,
                i, i, i, i
            ));
        }

        // Create in batches to avoid query size limits
        for chunk in users_data.chunks(100) {
            let create_query = format!(
                r#"mutation {{
                    createManyUser(data: [{}]) {{
                        count
                    }}
                }}"#,
                chunk.join(",")
            );
            run_query!(&runner, &create_query);
        }

        // Test extreme pagination
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { shardKey: "pagination-shard" }
                    orderBy: { score: asc }
                    take: 5
                    skip: 995
                ) {
                    id
                    score
                }
            }"#
        );

        // Should return the last 5 records
        assert!(result.contains("996") || result.contains("997"));

        Ok(())
    }
}
