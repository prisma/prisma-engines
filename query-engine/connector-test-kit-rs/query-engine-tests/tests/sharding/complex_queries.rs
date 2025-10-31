use query_engine_tests::*;

#[test_suite(only(MySql))]
mod shard_complex {
    use indoc::indoc;

    fn complex_schema() -> String {
        let schema = indoc! {
            r#"
            model User {
              id          String @id @default(uuid())
              email       String @unique
              username    String @unique
              firstName   String
              lastName    String
              age         Int?
              isActive    Boolean  @default(true)
              region      String   @shardKey
              score       Int      @default(0)
              balance     Decimal  @default(0.00)
              createdAt   DateTime @default(now())
              updatedAt   DateTime @updatedAt
              posts       Post[]
              comments    Comment[]
              profile     UserProfile?

              @@index([region, score])
              @@index([region, createdAt])
            }

            model UserProfile {
              id          String @id @default(uuid())
              userId      String @unique
              bio         String?
              website     String?
              avatar      String?
              country     String
              city        String
              region      String @shardKey
              user        User   @relation(fields: [userId], references: [id])
            }

            model Post {
              id          String   @id @default(uuid())
              title       String
              content     String
              authorId    String
              published   Boolean  @default(false)
              viewCount   Int      @default(0)
              likes       Int      @default(0)
              category    PostCategory
              region      String   @shardKey
              publishedAt DateTime?
              createdAt   DateTime @default(now())
              updatedAt   DateTime @updatedAt
              author      User     @relation(fields: [authorId], references: [id])
              comments    Comment[]
              tags        PostTag[]

              @@index([region, category])
              @@index([region, publishedAt])
              @@index([region, likes])
            }

            model Comment {
              id        String   @id @default(uuid())
              content   String
              authorId  String
              postId    String
              region    String   @shardKey
              createdAt DateTime @default(now())
              updatedAt DateTime @updatedAt
              author    User     @relation(fields: [authorId], references: [id])
              post      Post     @relation(fields: [postId], references: [id])

              @@index([region, postId])
              @@index([region, authorId])
            }

            model Tag {
              id    String @id @default(uuid())
              name  String @unique
              color String @default("black")
              posts PostTag[]
            }

            model PostTag {
              id     String @id @default(uuid())
              postId String
              tagId  String
              region String @shardKey
              post   Post   @relation(fields: [postId], references: [id])
              tag    Tag    @relation(fields: [tagId], references: [id])

              @@unique([postId, tagId])
              @@index([region, tagId])
            }

            model Analytics {
              id          String @id @default(uuid())
              eventType   String
              userId      String?
              postId      String?
              value       Int
              metadata    Json?
              region      String
              department  String
              timestamp   DateTime @default(now())

              @@shardKey([region, department])
              @@index([region, department, eventType])
              @@index([region, department, timestamp])
            }

            enum PostCategory {
              TECH
              BUSINESS
              HEALTH
              ENTERTAINMENT
              SPORTS
              SCIENCE
            }
            "#
        };

        schema.to_owned()
    }

    // Setup test data helper
    async fn setup_test_data(runner: &Runner) -> TestResult<()> {
        // Create users across different regions
        run_query!(
            runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-1"
                        email: "john@example.com"
                        username: "john_doe"
                        firstName: "John"
                        lastName: "Doe"
                        age: 25
                        region: "us-east-1"
                        score: 100
                        balance: 500.00
                    },
                    {
                        id: "user-2"
                        email: "jane@example.com"
                        username: "jane_smith"
                        firstName: "Jane"
                        lastName: "Smith"
                        age: 30
                        region: "us-west-2"
                        score: 200
                        balance: 750.50
                    },
                    {
                        id: "user-3"
                        email: "bob@example.com"
                        username: "bob_johnson"
                        firstName: "Bob"
                        lastName: "Johnson"
                        age: 35
                        region: "us-east-1"
                        score: 150
                        balance: 1000.75
                    },
                    {
                        id: "user-4"
                        email: "alice@example.com"
                        username: "alice_wilson"
                        firstName: "Alice"
                        lastName: "Wilson"
                        age: 28
                        region: "eu-west-1"
                        score: 300
                        balance: 2000.00
                    }
                ]) { count }
            }"#
        );

        // Create posts
        run_query!(
            runner,
            r#"mutation {
                createManyPost(data: [
                    {
                        id: "post-1"
                        title: "Tech Innovation"
                        content: "Latest in tech..."
                        authorId: "user-1"
                        published: true
                        viewCount: 100
                        likes: 25
                        category: TECH
                        region: "us-east-1"
                        publishedAt: "2024-01-01T00:00:00Z"
                    },
                    {
                        id: "post-2"
                        title: "Business Trends"
                        content: "Market analysis..."
                        authorId: "user-2"
                        published: true
                        viewCount: 200
                        likes: 50
                        category: BUSINESS
                        region: "us-west-2"
                        publishedAt: "2024-01-02T00:00:00Z"
                    },
                    {
                        id: "post-3"
                        title: "Health Tips"
                        content: "Stay healthy..."
                        authorId: "user-3"
                        published: false
                        viewCount: 50
                        likes: 10
                        category: HEALTH
                        region: "us-east-1"
                    },
                    {
                        id: "post-4"
                        title: "Entertainment News"
                        content: "Latest movies..."
                        authorId: "user-4"
                        published: true
                        viewCount: 300
                        likes: 75
                        category: ENTERTAINMENT
                        region: "eu-west-1"
                        publishedAt: "2024-01-03T00:00:00Z"
                    }
                ]) { count }
            }"#
        );

        // Create analytics data
        run_query!(
            runner,
            r#"mutation {
                createManyAnalytics(data: [
                    {
                        id: "analytics-1"
                        eventType: "page_view"
                        userId: "user-1"
                        postId: "post-1"
                        value: 1
                        region: "us-east-1"
                        department: "marketing"
                    },
                    {
                        id: "analytics-2"
                        eventType: "click"
                        userId: "user-2"
                        value: 5
                        region: "us-west-2"
                        department: "sales"
                    },
                    {
                        id: "analytics-3"
                        eventType: "conversion"
                        userId: "user-3"
                        value: 100
                        region: "us-east-1"
                        department: "marketing"
                    }
                ]) { count }
            }"#
        );

        Ok(())
    }

    // Complex Filtering Tests

    #[connector_test(schema(complex_schema))]
    async fn complex_where_conditions(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Complex query with multiple conditions including shard key
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(where: {
                    region: "us-east-1",
                    age: { gte: 25 },
                    score: { gte: 100 },
                    isActive: true
                }) {
                    id
                    firstName
                    age
                    score
                    region
                }
            }"#
        );

        assert!(result.contains("user-1"));
        assert!(result.contains("user-3"));
        assert!(!result.contains("user-2")); // Different region
        assert!(!result.contains("user-4")); // Different region

        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","age":25,"score":100,"region":"us-east-1"},{"id":"user-3","firstName":"Bob","age":35,"score":150,"region":"us-east-1"}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn or_conditions_across_shards(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // OR condition that spans multiple shards
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: {
                        OR: [
                            {
                                AND: [
                                    { region: "us-east-1" },
                                    { score: { gte: 150 } }
                                ]
                            },
                            {
                                AND: [
                                    { region: "eu-west-1" },
                                    { score: { gte: 250 } }
                                ]
                            }
                        ]
                    }
                    orderBy: { id: asc }
                ) {
                    id
                    firstName
                    score
                    region
                }
            }"#
        );

        assert!(result.contains("user-3")); // us-east-1 with score 150
        assert!(result.contains("user-4")); // eu-west-1 with score 300

        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-3","firstName":"Bob","score":150,"region":"us-east-1"},{"id":"user-4","firstName":"Alice","score":300,"region":"eu-west-1"}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn in_filter_with_shard_key(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // IN filter targeting specific shards
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: {
                        region: { in: ["us-east-1", "eu-west-1"] }
                        score: { in: [100, 300] }
                    }
                    orderBy: { id: asc }
                ) {
                    id
                    firstName
                    score
                    region
                }
            }"#
        );

        assert!(result.contains("user-1")); // us-east-1, score 100
        assert!(result.contains("user-4")); // eu-west-1, score 300

        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","score":100,"region":"us-east-1"},{"id":"user-4","firstName":"Alice","score":300,"region":"eu-west-1"}]}}"#);

        Ok(())
    }

    // Ordering and Pagination Tests

    #[connector_test(schema(complex_schema))]
    async fn orderby_with_shard_key_pagination(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Order by score within a shard with pagination
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { region: "us-east-1" }
                    orderBy: { score: desc }
                    take: 1
                    skip: 0
                ) {
                    id
                    firstName
                    score
                    region
                }
            }"#
        );

        // Highest score in us-east-1
        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-3","firstName":"Bob","score":150,"region":"us-east-1"}]}}"#);

        // Second page
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { region: "us-east-1" }
                    orderBy: { score: desc }
                    take: 1
                    skip: 1
                ) {
                    id
                    firstName
                    score
                    region
                }
            }"#
        );

        // Second highest score in us-east-1
        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","score":100,"region":"us-east-1"}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn complex_orderby_multiple_fields(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Complex ordering by multiple fields
        let result = run_query!(
            &runner,
            r#"query {
                findManyPost(
                    where: { region: "us-east-1" }
                    orderBy: [
                        { published: desc },
                        { likes: desc },
                        { createdAt: desc }
                    ]
                ) {
                    id
                    title
                    published
                    likes
                    region
                }
            }"#
        );

        // Should order published posts first, then by likes descending
        insta::assert_snapshot!(
            result,
            @r#"{"data":{"findManyPost":[{"id":"post-1","title":"Tech Innovation","published":true,"likes":25,"region":"us-east-1"},{"id":"post-3","title":"Health Tips","published":false,"likes":10,"region":"us-east-1"}]}}"#
        );

        Ok(())
    }

    // Aggregation Tests

    #[connector_test(schema(complex_schema))]
    async fn aggregate_by_shard_key(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Count users by region
        let result = run_query!(
            &runner,
            r#"query {
                aggregateUser(where: { region: "us-east-1" }) {
                    _count {
                        _all
                        id
                    }
                    _avg {
                        age
                        score
                        balance
                    }
                    _sum {
                        score
                        balance
                    }
                    _max {
                        age
                        score
                    }
                    _min {
                        age
                        score
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"aggregateUser":{"_count":{"_all":2,"id":2},"_avg":{"age":30,"score":125,"balance":"750.375"},"_sum":{"score":250,"balance":"1500.75"},"_max":{"age":35,"score":150},"_min":{"age":25,"score":100}}}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn group_by_with_shard_key(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Group analytics by region and department (composite shard key)
        let result = run_query!(
            &runner,
            r#"query {
                groupByAnalytics(
                    by: [region, department, eventType]
                ) {
                    region
                    department
                    eventType
                    _count {
                        _all
                        value
                    }
                    _sum {
                        value
                    }
                    _avg {
                        value
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"groupByAnalytics":[{"region":"us-east-1","department":"marketing","eventType":"conversion","_count":{"_all":1,"value":1},"_sum":{"value":100},"_avg":{"value":100}},{"region":"us-east-1","department":"marketing","eventType":"page_view","_count":{"_all":1,"value":1},"_sum":{"value":1},"_avg":{"value":1}},{"region":"us-west-2","department":"sales","eventType":"click","_count":{"_all":1,"value":1},"_sum":{"value":5},"_avg":{"value":5}}]}}"#);

        Ok(())
    }

    // Relation and Join Tests

    #[connector_test(schema(complex_schema))]
    async fn nested_shard_filtering(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Find users with their posts, filtering by shard key at multiple levels
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { region: "us-east-1" }
                ) {
                    id
                    firstName
                    region
                    posts(
                        where: {
                            region: "us-east-1"
                            published: true
                        }
                        orderBy: { likes: desc }
                    ) {
                        id
                        title
                        likes
                        region
                    }
                }
            }"#
        );

        assert!(result.contains("user-1"));
        assert!(result.contains("user-3"));
        assert!(result.contains("post-1")); // Published post by user-1

        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","region":"us-east-1","posts":[{"id":"post-1","title":"Tech Innovation","likes":25,"region":"us-east-1"}]},{"id":"user-3","firstName":"Bob","region":"us-east-1","posts":[]}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn complex_nested_relations_cross_shard(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Create comments across shards
        run_query!(
            &runner,
            r#"mutation {
                createManyComment(data: [
                    {
                        id: "comment-1"
                        content: "Great post!"
                        authorId: "user-2"
                        postId: "post-1"
                        region: "us-west-2"
                    },
                    {
                        id: "comment-2"
                        content: "Interesting perspective"
                        authorId: "user-1"
                        postId: "post-2"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Query posts with comments from different shards
        let result = run_query!(
            &runner,
            r#"query {
                findManyPost(
                    where: { published: true }
                ) {
                    id
                    title
                    region
                    author {
                        id
                        firstName
                        region
                    }
                    comments {
                        id
                        content
                        region
                        author {
                            firstName
                            region
                        }
                    }
                }
            }"#
        );

        assert!(result.contains("comment-1"));
        assert!(result.contains("comment-2"));

        insta::assert_snapshot!(result, @r#"{"data":{"findManyPost":[{"id":"post-1","title":"Tech Innovation","region":"us-east-1","author":{"id":"user-1","firstName":"John","region":"us-east-1"},"comments":[{"id":"comment-1","content":"Great post!","region":"us-west-2","author":{"firstName":"Jane","region":"us-west-2"}}]},{"id":"post-2","title":"Business Trends","region":"us-west-2","author":{"id":"user-2","firstName":"Jane","region":"us-west-2"},"comments":[{"id":"comment-2","content":"Interesting perspective","region":"us-east-1","author":{"firstName":"John","region":"us-east-1"}}]},{"id":"post-4","title":"Entertainment News","region":"eu-west-1","author":{"id":"user-4","firstName":"Alice","region":"eu-west-1"},"comments":[]}]}}"#);

        Ok(())
    }

    // Complex Filtering with Relations

    #[connector_test(schema(complex_schema))]
    async fn filter_by_related_model_shard_key(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Find posts where the author is in a specific region
        let result = run_query!(
            &runner,
            r#"query {
                findManyPost(where: {
                    author: {
                        region: "us-east-1"
                        score: { gte: 100 }
                    }
                    published: true
                }) {
                    id
                    title
                    region
                    author {
                        firstName
                        region
                        score
                    }
                }
            }"#
        );

        assert!(result.contains("post-1")); // By user-1 in us-east-1

        insta::assert_snapshot!(result, @r#"{"data":{"findManyPost":[{"id":"post-1","title":"Tech Innovation","region":"us-east-1","author":{"firstName":"John","region":"us-east-1","score":100}}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn some_none_filters_with_shard_keys(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Find users who have some posts in a specific region
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(where: {
                    posts: {
                        some: {
                            region: "us-east-1"
                            published: true
                        }
                    }
                }) {
                    id
                    firstName
                    region
                    posts(where: { region: "us-east-1" }) {
                        id
                        title
                        region
                    }
                }
            }"#
        );

        assert!(result.contains("user-1")); // Has published post in us-east-1

        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","region":"us-east-1","posts":[{"id":"post-1","title":"Tech Innovation","region":"us-east-1"}]}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn shard_aware_bulk_operations(runner: Runner) -> TestResult<()> {
        let mut users_data = Vec::new();
        let mut posts_data = Vec::new();

        for i in 1..=50 {
            let region = match i % 3 {
                0 => "us-east-1",
                1 => "us-west-2",
                _ => "eu-west-1",
            };

            users_data.push(format!(
                r#"{{
                    id: "bulk-user-{i}"
                    email: "user{i}@example.com"
                    username: "user{i}"
                    firstName: "User"
                    lastName: "{i}"
                    region: "{region}"
                    score: {}
                }}"#,
                i * 10
            ));

            posts_data.push(format!(
                r#"{{
                    id: "bulk-post-{i}"
                    title: "Post {i}"
                    content: "Content for post {i}"
                    authorId: "bulk-user-{i}"
                    region: "{region}"
                    category: TECH
                    published: true
                    likes: {}
                }}"#,
                i * 2
            ));
        }

        // Bulk create users
        let create_users_query = format!(
            r#"mutation {{
                createManyUser(data: [{}]) {{
                    count
                }}
            }}"#,
            users_data.join(",")
        );

        let user_result = run_query!(&runner, &create_users_query);
        insta::assert_snapshot!(user_result, @r#"{"data":{"createManyUser":{"count":50}}}"#);

        // Bulk create posts
        let create_posts_query = format!(
            r#"mutation {{
                createManyPost(data: [{}]) {{
                    count
                }}
            }}"#,
            posts_data.join(",")
        );

        let post_result = run_query!(&runner, &create_posts_query);
        insta::assert_snapshot!(post_result, @r#"{"data":{"createManyPost":{"count":50}}}"#);

        let query_result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { region: "us-east-1" }
                    orderBy: { score: desc }
                    take: 10
                ) {
                    id
                    score
                    region
                    posts(
                        where: { region: "us-east-1" }
                        orderBy: { likes: desc }
                        take: 3
                    ) {
                        id
                        likes
                        region
                    }
                }
            }"#
        );

        // Should return users only from us-east-1 shard
        assert!(query_result.contains("us-east-1"));
        assert!(!query_result.contains("us-west-2"));
        assert!(!query_result.contains("eu-west-1"));

        Ok(())
    }

    // Batch Operations

    #[connector_test(schema(complex_schema))]
    async fn batch_operations_within_shard(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Batch update users within the same shard
        let update_result = run_query!(
            &runner,
            r#"mutation {
                updateManyUser(
                    where: {
                        region: "us-east-1"
                        age: { gte: 30 }
                    }
                    data: {
                        score: { increment: 50 }
                        isActive: false
                    }
                ) {
                    count
                }
            }"#
        );

        // Only user-3 matches
        insta::assert_snapshot!(update_result, @r#"{"data":{"updateManyUser":{"count":1}}}"#);

        // Verify the update
        let verify_result = run_query!(
            &runner,
            r#"query {
                findUniqueUser(where: { id: "user-3" }) {
                    score
                    isActive
                    region
                }
            }"#
        );

        insta::assert_snapshot!(verify_result, @r#"{"data":{"findUniqueUser":{"score":200,"isActive":false,"region":"us-east-1"}}}"#);

        Ok(())
    }

    // Edge Cases and Complex Scenarios

    #[connector_test(schema(complex_schema))]
    async fn mixed_shard_and_non_shard_operations(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Query that mixes shard-aware and cross-shard operations
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: {
                        OR: [
                            { region: "us-east-1" },
                            { email: { contains: "jane" } }
                        ]
                    }
                    orderBy: { createdAt: desc }
                ) {
                    id
                    firstName
                    email
                    region
                    posts(
                        where: { published: true }
                        orderBy: { publishedAt: desc }
                    ) {
                        id
                        title
                        publishedAt
                        region
                    }
                }
            }"#
        );

        // Should include users from us-east-1 and Jane from us-west-2
        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","email":"john@example.com","region":"us-east-1","posts":[{"id":"post-1","title":"Tech Innovation","publishedAt":"2024-01-01T00:00:00.000Z","region":"us-east-1"}]},{"id":"user-2","firstName":"Jane","email":"jane@example.com","region":"us-west-2","posts":[{"id":"post-2","title":"Business Trends","publishedAt":"2024-01-02T00:00:00.000Z","region":"us-west-2"}]},{"id":"user-3","firstName":"Bob","email":"bob@example.com","region":"us-east-1","posts":[]}]}}"#);

        Ok(())
    }

    #[connector_test(schema(complex_schema))]
    async fn deeply_nested_relations(runner: Runner) -> TestResult<()> {
        setup_test_data(&runner).await?;

        // Create deeply nested data
        run_query!(
            &runner,
            r#"mutation {
                createOneUserProfile(data: {
                    id: "profile-1"
                    userId: "user-1"
                    bio: "Software Engineer"
                    country: "USA"
                    city: "New York"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Query with deep nesting across potential shard boundaries
        let result = run_query!(
            &runner,
            r#"query {
                findManyUser(
                    where: { region: "us-east-1" }
                ) {
                    id
                    firstName
                    region
                    profile {
                        bio
                        country
                        region
                    }
                    posts(
                        where: { published: true }
                    ) {
                        id
                        title
                        region
                        comments {
                            content
                            region
                            author {
                                firstName
                                region
                            }
                        }
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"findManyUser":[{"id":"user-1","firstName":"John","region":"us-east-1","profile":{"bio":"Software Engineer","country":"USA","region":"us-east-1"},"posts":[{"id":"post-1","title":"Tech Innovation","region":"us-east-1","comments":[]}]},{"id":"user-3","firstName":"Bob","region":"us-east-1","profile":null,"posts":[]}]}}"#);

        Ok(())
    }
}
