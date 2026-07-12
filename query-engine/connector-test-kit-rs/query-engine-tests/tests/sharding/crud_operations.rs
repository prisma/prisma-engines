use query_engine_tests::*;

#[test_suite(only(MySql))]
mod shard_crud {
    use indoc::indoc;

    fn crud_schema() -> String {
        let schema = indoc! {
            r#"
            model Customer {
              id          String @id @default(uuid())
              email       String @unique
              firstName   String
              lastName    String
              region      String @shardKey
              createdAt   DateTime @default(now())
              updatedAt   DateTime @updatedAt
              orders      Order[]
              profile     CustomerProfile?
            }

            model CustomerProfile {
              id          String @id @default(uuid())
              customerId  String @unique
              bio         String?
              avatar      String?
              region      String @shardKey
              customer    Customer @relation(fields: [customerId], references: [id])
            }

            model Order {
              id          String @id @default(uuid())
              orderNumber String @unique
              customerId  String
              totalAmount Decimal
              status      OrderStatus @default(PENDING)
              region      String @shardKey
              orderDate   DateTime @default(now())
              customer    Customer @relation(fields: [customerId], references: [id])
              items       OrderItem[]
            }

            model OrderItem {
              id          String @id @default(uuid())
              orderId     String
              productName String
              quantity    Int
              price       Decimal
              region      String @shardKey
              order       Order @relation(fields: [orderId], references: [id])
            }

            model Product {
              id          String @id @default(uuid())
              sku         String @unique
              name        String
              description String?
              price       Decimal
              category    String
              region      String

              @@shardKey([region, category])
            }

            enum OrderStatus {
              PENDING
              CONFIRMED
              SHIPPED
              DELIVERED
              CANCELLED
            }
            "#
        };

        schema.to_owned()
    }

    // CREATE Operations Tests

    #[connector_test(schema(crud_schema))]
    async fn create_one_customer(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john.doe@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                }) {
                    id
                    email
                    firstName
                    lastName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"createOneCustomer":{"id":"customer-1","email":"john.doe@example.com","firstName":"John","lastName":"Doe","region":"us-east-1"}}}"#);

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn create_many_customers(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createManyCustomer(data: [
                    {
                        id: "customer-1"
                        email: "john@example.com"
                        firstName: "John"
                        lastName: "Doe"
                        region: "us-east-1"
                    },
                    {
                        id: "customer-2"
                        email: "jane@example.com"
                        firstName: "Jane"
                        lastName: "Smith"
                        region: "us-west-2"
                    },
                    {
                        id: "customer-3"
                        email: "bob@example.com"
                        firstName: "Bob"
                        lastName: "Johnson"
                        region: "us-east-1"
                    }
                ]) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createManyCustomer":{"count":3}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn create_customer_with_nested_profile(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                    profile: {
                        create: {
                            id: "profile-1"
                            bio: "Software Engineer"
                            avatar: "avatar.jpg"
                            region: "us-east-1"
                        }
                    }
                }) {
                    id
                    email
                    region
                    profile {
                        id
                        bio
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"createOneCustomer":{"id":"customer-1","email":"john@example.com","region":"us-east-1","profile":{"id":"profile-1","bio":"Software Engineer","region":"us-east-1"}}}}"#);

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn create_composite_shard_key_product(runner: Runner) -> TestResult<()> {
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneProduct(data: {
                    id: "product-1"
                    sku: "SKU-001"
                    name: "Laptop"
                    description: "High-performance laptop"
                    price: 999.99
                    category: "electronics"
                    region: "us-east-1"
                }) {
                    id
                    sku
                    name
                    category
                    region
                    price
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneProduct":{"id":"product-1","sku":"SKU-001","name":"Laptop","category":"electronics","region":"us-east-1","price":"999.99"}}}"###
        );

        Ok(())
    }

    // READ Operations Tests

    #[connector_test(schema(crud_schema))]
    async fn find_unique_customer_by_id(runner: Runner) -> TestResult<()> {
        // Setup: Create customer
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Test: Find by ID (this will use shard-aware primary identifier)
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueCustomer(where: { id: "customer-1" }) {
                    id
                    email
                    firstName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findUniqueCustomer":{"id":"customer-1","email":"john@example.com","firstName":"John","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn find_unique_customer_by_email(runner: Runner) -> TestResult<()> {
        // Setup: Create customer
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Test: Find by unique email
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueCustomer(where: { email: "john@example.com" }) {
                    id
                    email
                    firstName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findUniqueCustomer":{"id":"customer-1","email":"john@example.com","firstName":"John","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn find_many_customers_by_shard_key(runner: Runner) -> TestResult<()> {
        // Setup: Create customers in different regions
        run_query!(
            &runner,
            r#"mutation {
                createManyCustomer(data: [
                    {
                        id: "customer-1"
                        email: "john@example.com"
                        firstName: "John"
                        lastName: "Doe"
                        region: "us-east-1"
                    },
                    {
                        id: "customer-2"
                        email: "jane@example.com"
                        firstName: "Jane"
                        lastName: "Smith"
                        region: "us-west-2"
                    },
                    {
                        id: "customer-3"
                        email: "bob@example.com"
                        firstName: "Bob"
                        lastName: "Johnson"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Test: Find customers by shard key (should be efficient single-shard query)
        let result = run_query!(
            &runner,
            r#"query {
                findManyCustomer(where: { region: "us-east-1" }) {
                    id
                    firstName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"findManyCustomer":[{"id":"customer-1","firstName":"John","region":"us-east-1"},{"id":"customer-3","firstName":"Bob","region":"us-east-1"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn find_first_customer_with_shard_filter(runner: Runner) -> TestResult<()> {
        // Setup: Create customers
        run_query!(
            &runner,
            r#"mutation {
                createManyCustomer(data: [
                    {
                        id: "customer-1"
                        email: "john@example.com"
                        firstName: "John"
                        lastName: "Doe"
                        region: "us-east-1"
                    },
                    {
                        id: "customer-2"
                        email: "jane@example.com"
                        firstName: "Jane"
                        lastName: "Smith"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Test: Find first customer in region
        let result = run_query!(
            &runner,
            r#"query {
                findFirstCustomer(where: { region: "us-east-1" }) {
                    id
                    firstName
                    region
                }
            }"#
        );

        // Should return one of the customers in us-east-1
        insta::assert_snapshot!(result, @r#"{"data":{"findFirstCustomer":{"id":"customer-1","firstName":"John","region":"us-east-1"}}}"#);

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn find_customer_with_nested_relations(runner: Runner) -> TestResult<()> {
        // Setup: Create customer with profile and orders
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                    profile: {
                        create: {
                            id: "profile-1"
                            bio: "Software Engineer"
                            region: "us-east-1"
                        }
                    }
                    orders: {
                        create: [
                            {
                                id: "order-1"
                                orderNumber: "ORD-001"
                                totalAmount: 100.50
                                region: "us-east-1"
                            },
                            {
                                id: "order-2"
                                orderNumber: "ORD-002"
                                totalAmount: 250.75
                                region: "us-east-1"
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // Test: Find customer with all relations
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueCustomer(where: { id: "customer-1" }) {
                    id
                    firstName
                    region
                    profile {
                        id
                        bio
                        region
                    }
                    orders {
                        id
                        orderNumber
                        totalAmount
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"findUniqueCustomer":{"id":"customer-1","firstName":"John","region":"us-east-1","profile":{"id":"profile-1","bio":"Software Engineer","region":"us-east-1"},"orders":[{"id":"order-1","orderNumber":"ORD-001","totalAmount":"100.5","region":"us-east-1"},{"id":"order-2","orderNumber":"ORD-002","totalAmount":"250.75","region":"us-east-1"}]}}}"#);

        Ok(())
    }

    // UPDATE Operations Tests

    #[connector_test(schema(crud_schema))]
    async fn update_one_customer(runner: Runner) -> TestResult<()> {
        // Setup: Create customer
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Test: Update customer (should use shard-aware primary identifier)
        let result = run_query!(
            &runner,
            r#"mutation {
                updateOneCustomer(
                    where: { id: "customer-1" }
                    data: {
                        firstName: "Johnny"
                        lastName: "Updated"
                    }
                ) {
                    id
                    firstName
                    lastName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"updateOneCustomer":{"id":"customer-1","firstName":"Johnny","lastName":"Updated","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn update_many_customers_by_shard(runner: Runner) -> TestResult<()> {
        // Setup: Create customers in different regions
        run_query!(
            &runner,
            r#"mutation {
                createManyCustomer(data: [
                    {
                        id: "customer-1"
                        email: "john@example.com"
                        firstName: "John"
                        lastName: "Doe"
                        region: "us-east-1"
                    },
                    {
                        id: "customer-2"
                        email: "jane@example.com"
                        firstName: "Jane"
                        lastName: "Smith"
                        region: "us-west-2"
                    },
                    {
                        id: "customer-3"
                        email: "bob@example.com"
                        firstName: "Bob"
                        lastName: "Johnson"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Test: Update all customers in a specific region
        let result = run_query!(
            &runner,
            r#"mutation {
                updateManyCustomer(
                    where: { region: "us-east-1" }
                    data: { lastName: "UpdatedInEast" }
                ) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"updateManyCustomer":{"count":2}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn upsert_customer_create(runner: Runner) -> TestResult<()> {
        // Test: Upsert that creates a new record
        let result = run_query!(
            &runner,
            r#"mutation {
                upsertOneCustomer(
                    where: { email: "new@example.com" }
                    create: {
                        id: "customer-1"
                        email: "new@example.com"
                        firstName: "New"
                        lastName: "Customer"
                        region: "us-east-1"
                    }
                    update: {
                        firstName: "Updated"
                    }
                ) {
                    id
                    email
                    firstName
                    lastName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"upsertOneCustomer":{"id":"customer-1","email":"new@example.com","firstName":"New","lastName":"Customer","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn upsert_customer_update(runner: Runner) -> TestResult<()> {
        // Setup: Create existing customer
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "existing@example.com"
                    firstName: "Existing"
                    lastName: "Customer"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Test: Upsert that updates existing record
        let result = run_query!(
            &runner,
            r#"mutation {
                upsertOneCustomer(
                    where: { email: "existing@example.com" }
                    create: {
                        id: "customer-new"
                        email: "existing@example.com"
                        firstName: "Should Not Create"
                        lastName: "Should Not Create"
                        region: "us-west-2"
                    }
                    update: {
                        firstName: "Updated"
                        lastName: "Existing"
                    }
                ) {
                    id
                    email
                    firstName
                    lastName
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"upsertOneCustomer":{"id":"customer-1","email":"existing@example.com","firstName":"Updated","lastName":"Existing","region":"us-east-1"}}}"###
        );

        Ok(())
    }

    // DELETE Operations Tests

    #[connector_test(schema(crud_schema))]
    async fn delete_one_customer(runner: Runner) -> TestResult<()> {
        // Setup: Create customer
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Test: Delete customer (should use shard-aware primary identifier)
        let result = run_query!(
            &runner,
            r#"mutation {
                deleteOneCustomer(where: { id: "customer-1" }) {
                    id
                    email
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"deleteOneCustomer":{"id":"customer-1","email":"john@example.com","region":"us-east-1"}}}"###
        );

        // Verify deletion
        let verify_result = run_query!(
            &runner,
            r#"query {
                findUniqueCustomer(where: { id: "customer-1" }) {
                    id
                }
            }"#
        );

        insta::assert_snapshot!(
            verify_result,
            @r###"{"data":{"findUniqueCustomer":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn delete_many_customers_by_shard(runner: Runner) -> TestResult<()> {
        // Setup: Create customers in different regions
        run_query!(
            &runner,
            r#"mutation {
                createManyCustomer(data: [
                    {
                        id: "customer-1"
                        email: "john@example.com"
                        firstName: "John"
                        lastName: "Doe"
                        region: "us-east-1"
                    },
                    {
                        id: "customer-2"
                        email: "jane@example.com"
                        firstName: "Jane"
                        lastName: "Smith"
                        region: "us-west-2"
                    },
                    {
                        id: "customer-3"
                        email: "bob@example.com"
                        firstName: "Bob"
                        lastName: "Johnson"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Test: Delete all customers in a specific region
        let result = run_query!(
            &runner,
            r#"mutation {
                deleteManyCustomer(where: { region: "us-east-1" }) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"deleteManyCustomer":{"count":2}}}"###
        );

        // Verify only one customer remains
        let verify_result = run_query!(
            &runner,
            r#"query {
                findManyCustomer {
                    id
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            verify_result,
            @r###"{"data":{"findManyCustomer":[{"id":"customer-2","region":"us-west-2"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(crud_schema))]
    async fn delete_customer_with_cascade_relations(runner: Runner) -> TestResult<()> {
        // Setup: Create customer with profile and orders
        run_query!(
            &runner,
            r#"mutation {
                createOneCustomer(data: {
                    id: "customer-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                    orders: {
                        create: [
                            {
                                id: "order-1"
                                orderNumber: "ORD-001"
                                totalAmount: 100.50
                                region: "us-east-1"
                                items: {
                                    create: [
                                        {
                                            id: "item-1"
                                            productName: "Laptop"
                                            quantity: 1
                                            price: 100.50
                                            region: "us-east-1"
                                        }
                                    ]
                                }
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // First delete related items
        run_query!(
            &runner,
            r#"mutation {
                deleteManyOrderItem(where: {
                    order: { customerId: "customer-1" }
                }) {
                    count
                }
            }"#
        );

        // Then delete orders
        run_query!(
            &runner,
            r#"mutation {
                deleteManyOrder(where: { customerId: "customer-1" }) {
                    count
                }
            }"#
        );

        // Finally delete customer
        let result = run_query!(
            &runner,
            r#"mutation {
                deleteOneCustomer(where: { id: "customer-1" }) {
                    id
                    email
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"deleteOneCustomer":{"id":"customer-1","email":"john@example.com"}}}"###
        );

        Ok(())
    }

    // Complex Operations with Composite Shard Keys

    #[connector_test(schema(crud_schema))]
    async fn composite_shard_key_product_ops(runner: Runner) -> TestResult<()> {
        // Create product with composite shard key
        run_query!(
            &runner,
            r#"mutation {
                createOneProduct(data: {
                    id: "product-1"
                    sku: "SKU-001"
                    name: "Laptop"
                    description: "High-performance laptop"
                    price: 999.99
                    category: "electronics"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Update product
        let update_result = run_query!(
            &runner,
            r#"mutation {
                updateOneProduct(
                    where: { id: "product-1" }
                    data: {
                        name: "Gaming Laptop"
                        price: 1299.99
                    }
                ) {
                    id
                    name
                    price
                    category
                    region
                }
            }"#
        );

        insta::assert_snapshot!(
            update_result,
            @r###"{"data":{"updateOneProduct":{"id":"product-1","name":"Gaming Laptop","price":"1299.99","category":"electronics","region":"us-east-1"}}}"###
        );

        // Find by composite shard key fields
        let find_result = run_query!(
            &runner,
            r#"query {
                findManyProduct(where: {
                    region: "us-east-1"
                    category: "electronics"
                }) {
                    id
                    name
                    category
                    region
                }
            }"#
        );

        insta::assert_snapshot!(find_result, @r#"{"data":{"findManyProduct":[{"id":"product-1","name":"Gaming Laptop","category":"electronics","region":"us-east-1"}]}}"#);

        // Delete product
        let delete_result = run_query!(
            &runner,
            r#"mutation {
                deleteOneProduct(where: { id: "product-1" }) {
                    id
                    name
                }
            }"#
        );

        insta::assert_snapshot!(
            delete_result,
            @r###"{"data":{"deleteOneProduct":{"id":"product-1","name":"Gaming Laptop"}}}"###
        );

        Ok(())
    }
}
