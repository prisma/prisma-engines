use query_engine_tests::*;

#[test_suite(only(MySql))]
mod basic_shard_key {
    use indoc::indoc;

    fn single_shard_key_schema() -> String {
        let schema = indoc! {
            r#"
            model User {
              id       String @id @default(uuid())
              email    String @unique
              name     String
              region   String @shardKey
              posts    Post[]
            }

            model Post {
              id       String @id @default(uuid())
              title    String
              content  String
              userId   String
              region   String @shardKey
              user     User   @relation(fields: [userId], references: [id])
            }
            "#
        };

        schema.to_owned()
    }

    fn composite_shard_key_schema() -> String {
        let schema = indoc! {
            r#"
            model Order {
              id         String @id @default(uuid())
              customerId String
              productId  String
              quantity   Int
              region     String
              tenantId   String

              @@shardKey([region, tenantId])
            }

            model Product {
              id       String @id @default(uuid())
              name     String
              price    Decimal
              region   String
              category String

              @@shardKey([region, category])
            }
            "#
        };

        schema.to_owned()
    }

    fn mixed_key_schema() -> String {
        let schema = indoc! {
            r#"
            model Document {
              id       String @id @default(uuid())
              title    String
              content  String
              shardKey String @shardKey
            }

            model CompoundPrimary {
              userId   String
              gameId   String
              score    Int
              region   String

              @@id([userId, gameId])
              @@shardKey([region])
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn create_record_with_single_shard_key(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "test@example.com"
                    name: "Test User"
                    region: "us-east-1"
                }) {
                    id
                    email
                    name
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneUser":{"id":"user-1","email":"test@example.com","name":"Test User","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn find_unique_with_shard_key(runner: Runner) -> TestResult<()> {
        // First create a record
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "test@example.com"
                    name: "Test User"
                    region: "us-east-1"
                }) {
                    id
                }
            }"#
        );

        // Find by primary key only (should work but might be cross-shard)
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueUser(where: { id: "user-1" }) {
                    id
                    email
                    name
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findUniqueUser":{"id":"user-1","email":"test@example.com","name":"Test User","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn find_unique_with_email_unique_index(runner: Runner) -> TestResult<()> {
        // First create a record
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "test@example.com"
                    name: "Test User"
                    region: "us-east-1"
                }) {
                    id
                }
            }"#
        );

        // Find by unique email field
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueUser(where: { email: "test@example.com" }) {
                    id
                    email
                    name
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findUniqueUser":{"id":"user-1","email":"test@example.com","name":"Test User","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn update_record_with_shard_key(runner: Runner) -> TestResult<()> {
        // First create a record
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "test@example.com"
                    name: "Test User"
                    region: "us-east-1"
                }) {
                    id
                }
            }"#
        );

        // Update the record
        let result = run_query!(
            &runner,
            r#"mutation {
                updateOneUser(
                    where: { id: "user-1" }
                    data: { name: "Updated User" }
                ) {
                    id
                    email
                    name
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"updateOneUser":{"id":"user-1","email":"test@example.com","name":"Updated User","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn delete_record_with_shard_key(runner: Runner) -> TestResult<()> {
        // First create a record
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "test@example.com"
                    name: "Test User"
                    region: "us-east-1"
                }) {
                    id
                }
            }"#
        );

        // Delete the record
        let result = run_query!(
            &runner,
            r#"mutation {
                deleteOneUser(where: { id: "user-1" }) {
                    id
                    email
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"deleteOneUser":{"id":"user-1","email":"test@example.com","region":"us-east-1"}}}"###
        );

        // Verify it's deleted
        let verify_result = run_query!(
            &runner,
            r#"query {
                findUniqueUser(where: { id: "user-1" }) {
                    id
                }
            }"#
        );

        insta::assert_snapshot!(
            verify_result,
            @r###"{"data":{"findUniqueUser":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(composite_shard_key_schema))]
    async fn create_record_with_composite_shard_key(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneOrder(data: {
                    id: "order-1"
                    customerId: "customer-1"
                    productId: "product-1"
                    quantity: 5
                    region: "us-west-2"
                    tenantId: "tenant-a"
                }) {
                    id
                    customerId
                    productId
                    quantity
                    region
                    tenantId
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneOrder":{"id":"order-1","customerId":"customer-1","productId":"product-1","quantity":5,"region":"us-west-2","tenantId":"tenant-a"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(composite_shard_key_schema))]
    async fn update_record_with_composite_shard_key(runner: Runner) -> TestResult<()> {
        // First create a record
        run_query!(
            &runner,
            r#"mutation {
                createOneOrder(data: {
                    id: "order-1"
                    customerId: "customer-1"
                    productId: "product-1"
                    quantity: 5
                    region: "us-west-2"
                    tenantId: "tenant-a"
                }) {
                    id
                }
            }"#
        );

        // Update the record
        let result = run_query!(
            &runner,
            r#"mutation {
                updateOneOrder(
                    where: { id: "order-1" }
                    data: { quantity: 10 }
                ) {
                    id
                    customerId
                    productId
                    quantity
                    region
                    tenantId
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"updateOneOrder":{"id":"order-1","customerId":"customer-1","productId":"product-1","quantity":10,"region":"us-west-2","tenantId":"tenant-a"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(mixed_key_schema))]
    async fn create_compound_primary_shard_key(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneCompoundPrimary(data: {
                    userId: "user-1"
                    gameId: "game-1"
                    score: 1000
                    region: "eu-central-1"
                }) {
                    userId
                    gameId
                    score
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneCompoundPrimary":{"userId":"user-1","gameId":"game-1","score":1000,"region":"eu-central-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(mixed_key_schema))]
    async fn find_compound_primary_shard_key(runner: Runner) -> TestResult<()> {
        // First create a record
        run_query!(
            &runner,
            r#"mutation {
                createOneCompoundPrimary(data: {
                    userId: "user-1"
                    gameId: "game-1"
                    score: 1000
                    region: "eu-central-1"
                }) {
                    userId
                    gameId
                }
            }"#
        );

        // Find by compound primary key
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueCompoundPrimary(where: {
                    userId_gameId: { userId: "user-1", gameId: "game-1" }
                }) {
                    userId
                    gameId
                    score
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findUniqueCompoundPrimary":{"userId":"user-1","gameId":"game-1","score":1000,"region":"eu-central-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn create_many_with_shard_key(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-1"
                        email: "user1@example.com"
                        name: "User One"
                        region: "us-east-1"
                    },
                    {
                        id: "user-2"
                        email: "user2@example.com"
                        name: "User Two"
                        region: "us-west-2"
                    },
                    {
                        id: "user-3"
                        email: "user3@example.com"
                        name: "User Three"
                        region: "us-east-1"
                    }
                ]) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createManyUser":{"count":3}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn find_many_with_shard_key_filter(runner: Runner) -> TestResult<()> {
        // First create test data
        run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-1"
                        email: "user1@example.com"
                        name: "User One"
                        region: "us-east-1"
                    },
                    {
                        id: "user-2"
                        email: "user2@example.com"
                        name: "User Two"
                        region: "us-west-2"
                    },
                    {
                        id: "user-3"
                        email: "user3@example.com"
                        name: "User Three"
                        region: "us-east-1"
                    }
                ]) {
                    count
                }
            }"#
        );

        // Find users by shard key - this should be efficient as it targets a specific shard
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(where: { region: "us-east-1" }) {
                    id
                    email
                    name
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findManyUser":[{"id":"user-1","email":"user1@example.com","name":"User One","region":"us-east-1"},{"id":"user-3","email":"user3@example.com","name":"User Three","region":"us-east-1"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn update_many_with_shard_key_filter(runner: Runner) -> TestResult<()> {
        // First create test data
        run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-1"
                        email: "user1@example.com"
                        name: "User One"
                        region: "us-east-1"
                    },
                    {
                        id: "user-2"
                        email: "user2@example.com"
                        name: "User Two"
                        region: "us-west-2"
                    },
                    {
                        id: "user-3"
                        email: "user3@example.com"
                        name: "User Three"
                        region: "us-east-1"
                    }
                ]) {
                    count
                }
            }"#
        );

        // Update users in a specific region
        let result = run_query!(
            &runner,
            r#"mutation {
                updateManyUser(
                    where: { region: "us-east-1" }
                    data: { name: "Updated User" }
                ) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"updateManyUser":{"count":2}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(single_shard_key_schema))]
    async fn delete_many_with_shard_key_filter(runner: Runner) -> TestResult<()> {
        // First create test data
        run_query!(
            &runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-1"
                        email: "user1@example.com"
                        name: "User One"
                        region: "us-east-1"
                    },
                    {
                        id: "user-2"
                        email: "user2@example.com"
                        name: "User Two"
                        region: "us-west-2"
                    },
                    {
                        id: "user-3"
                        email: "user3@example.com"
                        name: "User Three"
                        region: "us-east-1"
                    }
                ]) {
                    count
                }
            }"#
        );

        // Delete users in a specific region
        let result = run_query!(
            &runner,
            r#"mutation {
                deleteManyUser(where: { region: "us-east-1" }) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"deleteManyUser":{"count":2}}}"###
        );

        // Verify only one user remains
        let verify_result = run_query!(
            &runner,
            r#"query {
                findManyUser {
                    id
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            verify_result,
            @r###"{"data":{"findManyUser":[{"id":"user-2","region":"us-west-2"}]}}"###
        );

        Ok(())
    }
}
