use query_engine_tests::*;

/// Tests complex relational queries with `@shardKey` and `relationMode = "prisma"`.
#[test_suite(only(MySql), relation_mode = "prisma")]
mod relations_shard_key {
    use indoc::indoc;
    use query_engine_tests::{Runner, run_query};

    fn relations_schema() -> String {
        let schema = indoc! {
            r##"
            // One-to-One Relations
            model User {
              id               String       @id @default(uuid())
              email            String       @unique
              firstName        String
              lastName         String
              region           String       @shardKey
              profile          UserProfile?
              posts            Post[]
              authoredComments Comment[]    @relation("AuthorComments")
              receivedComments Comment[]    @relation("PostOwnerComments")
            }

            model UserProfile {
              id        String  @id @default(uuid())
              userId    String  @unique
              bio       String?
              website   String?
              avatarUrl String?
              region    String  @shardKey
              user      User    @relation(fields: [userId], references: [id])
            }

            // One-to-Many Relations
            model Post {
              id        String    @id @default(uuid())
              title     String
              content   String
              published Boolean   @default(false)
              authorId  String
              region    String    @shardKey
              author    User      @relation(fields: [authorId], references: [id])
              comments  Comment[]
              postTags  PostTag[]
            }

            model Comment {
              id        String @id @default(uuid())
              content   String
              authorId  String
              postId    String
              region    String @shardKey
              author    User   @relation("AuthorComments", fields: [authorId], references: [id])
              post      Post   @relation(fields: [postId], references: [id], map: "Comment_post_fkey")
              postOwner User   @relation("PostOwnerComments", fields: [postId], references: [id], map: "Comment_postOwner_fkey")
            }

            // Explicit Many-to-Many Relations
            model Tag {
              id       String    @id @default(uuid())
              name     String    @unique
              color    String    @default("000000")
              postTags PostTag[]
            }

            model PostTag {
              id     String @id @default(uuid())
              postId String
              tagId  String
              region String @shardKey
              post   Post   @relation(fields: [postId], references: [id])
              tag    Tag    @relation(fields: [tagId], references: [id])

              @@unique([postId, tagId])
            }

            // Cross-Shard Relations
            model Organization {
              id          String       @id @default(uuid())
              name        String
              region      String       @shardKey
              departments Department[]
              employees   Employee[]
            }

            model Department {
              id             String       @id @default(uuid())
              name           String
              organizationId String
              region         String       @shardKey
              organization   Organization @relation(fields: [organizationId], references: [id])
              employees      Employee[]
            }

            model Employee {
              id             String       @id @default(uuid())
              firstName      String
              lastName       String
              email          String       @unique
              organizationId String
              departmentId   String?
              region         String       @shardKey
              organization   Organization @relation(fields: [organizationId], references: [id])
              department     Department?  @relation(fields: [departmentId], references: [id])
            }

            // Composite Shard Key Relations
            model Tenant {
              id       String    @id @default(uuid())
              name     String
              region   String
              tier     String
              projects Project[]

              @@shardKey([region, tier])
            }

            model Project {
              id       String @id @default(uuid())
              name     String
              tenantId String
              region   String
              category String
              tenant   Tenant @relation(fields: [tenantId], references: [id])
              tasks    Task[]

              @@shardKey([region, category])
            }

            model Task {
              id        String  @id @default(uuid())
              title     String
              completed Boolean @default(false)
              projectId String
              region    String
              priority  String
              project   Project @relation(fields: [projectId], references: [id])

              @@shardKey([region, priority])
            }
            "##
        };

        schema.to_owned()
    }

    // Setup helper for test data
    async fn setup_relation_test_data(runner: &Runner) -> TestResult<()> {
        // Create users
        run_query!(
            runner,
            r#"mutation {
                createManyUser(data: [
                    {
                        id: "user-1"
                        email: "john@example.com"
                        firstName: "John"
                        lastName: "Doe"
                        region: "us-east-1"
                    },
                    {
                        id: "user-2"
                        email: "jane@example.com"
                        firstName: "Jane"
                        lastName: "Smith"
                        region: "us-west-2"
                    },
                    {
                        id: "user-3"
                        email: "bob@example.com"
                        firstName: "Bob"
                        lastName: "Johnson"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Create tags
        run_query!(
            runner,
            r##"mutation {
                createManyTag(data: [
                    {
                        id: "tag-1"
                        name: "technology"
                        color: "#0066CC"
                    },
                    {
                        id: "tag-2"
                        name: "business"
                        color: "#CC6600"
                    },
                    {
                        id: "tag-3"
                        name: "science"
                        color: "#00CC66"
                    }
                ]) { count }
            }"##
        );

        Ok(())
    }

    // One-to-One Relation Tests

    #[connector_test(schema(relations_schema))]
    async fn create_one_to_one_relation_same_shard(runner: Runner) -> TestResult<()> {
        // Create user with profile in the same shard
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                    profile: {
                        create: {
                            id: "profile-1"
                            bio: "Software Engineer"
                            website: "https://johndoe.dev"
                            region: "us-east-1"
                        }
                    }
                }) {
                    id
                    firstName
                    region
                    profile {
                        id
                        bio
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneUser":{"id":"user-1","firstName":"John","region":"us-east-1","profile":{"id":"profile-1","bio":"Software Engineer","region":"us-east-1"}}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn create_one_to_one_relation_different_shards(runner: Runner) -> TestResult<()> {
        // Create user
        run_query!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Create profile in different shard
        let result = run_query!(
            &runner,
            r#"mutation {
                createOneUserProfile(data: {
                    id: "profile-1"
                    userId: "user-1"
                    bio: "Software Engineer"
                    region: "us-west-2"
                }) {
                    id
                    bio
                    region
                    user {
                        firstName
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createOneUserProfile":{"id":"profile-1","bio":"Software Engineer","region":"us-west-2","user":{"firstName":"John","region":"us-east-1"}}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn query_one_to_one_with_include(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create profile
        run_query!(
            &runner,
            r#"mutation {
                createOneUserProfile(data: {
                    id: "profile-1"
                    userId: "user-1"
                    bio: "Software Engineer"
                    website: "https://johndoe.dev"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Query user with profile
        let result = run_query!(
            &runner,
            r#"query {
                findUniqueUser(where: { id: "user-1" }) {
                    id
                    firstName
                    region
                    profile {
                        bio
                        website
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"findUniqueUser":{"id":"user-1","firstName":"John","region":"us-east-1","profile":{"bio":"Software Engineer","website":"https://johndoe.dev","region":"us-east-1"}}}}"#);

        Ok(())
    }

    // One-to-Many Relation Tests

    #[connector_test(schema(relations_schema))]
    async fn create_one_to_many_relation_same_shard(runner: Runner) -> TestResult<()> {
        // Create user with posts in the same shard
        let result = run_query_pretty!(
            &runner,
            r#"mutation {
                createOneUser(data: {
                    id: "user-1"
                    email: "john@example.com"
                    firstName: "John"
                    lastName: "Doe"
                    region: "us-east-1"
                    posts: {
                        create: [
                            {
                                id: "post-1"
                                title: "First Post"
                                content: "Hello World"
                                region: "us-east-1"
                            },
                            {
                                id: "post-2"
                                title: "Second Post"
                                content: "Another post"
                                published: true
                                region: "us-east-1"
                            }
                        ]
                    }
                }) {
                    id
                    firstName
                    region
                    posts {
                        id
                        title
                        published
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "createOneUser": {
              "id": "user-1",
              "firstName": "John",
              "region": "us-east-1",
              "posts": [
                {
                  "id": "post-1",
                  "title": "First Post",
                  "published": false,
                  "region": "us-east-1"
                },
                {
                  "id": "post-2",
                  "title": "Second Post",
                  "published": true,
                  "region": "us-east-1"
                }
              ]
            }
          }
        }
        "#);

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn create_posts_across_different_shards(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create posts for user-1 in different shards
        run_query!(
            &runner,
            r#"mutation {
                createManyPost(data: [
                    {
                        id: "post-1"
                        title: "East Coast Post"
                        content: "Content from east"
                        authorId: "user-1"
                        region: "us-east-1"
                    },
                    {
                        id: "post-2"
                        title: "West Coast Post"
                        content: "Content from west"
                        authorId: "user-1"
                        region: "us-west-2"
                    }
                ]) { count }
            }"#
        );

        // Query user with posts from all shards
        let result = run_query_pretty!(
            &runner,
            r#"query {
                findUniqueUser(where: { id: "user-1" }) {
                    id
                    firstName
                    region
                    posts {
                        id
                        title
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "findUniqueUser": {
              "id": "user-1",
              "firstName": "John",
              "region": "us-east-1",
              "posts": [
                {
                  "id": "post-1",
                  "title": "East Coast Post",
                  "region": "us-east-1"
                },
                {
                  "id": "post-2",
                  "title": "West Coast Post",
                  "region": "us-west-2"
                }
              ]
            }
          }
        }
        "#);

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn nested_create_with_comments(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create post with comments
        let result = run_query_pretty!(
            &runner,
            r#"mutation {
                createOnePost(data: {
                    id: "post-1"
                    title: "Discussion Post"
                    content: "Let's discuss this topic"
                    authorId: "user-1"
                    published: true
                    region: "us-east-1"
                    comments: {
                        create: [
                            {
                                id: "comment-1"
                                content: "Great post!"
                                authorId: "user-2"
                                region: "us-west-2"
                            },
                            {
                                id: "comment-2"
                                content: "I agree!"
                                authorId: "user-3"
                                region: "us-east-1"
                            }
                        ]
                    }
                }) {
                    id
                    title
                    region
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

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "createOnePost": {
              "id": "post-1",
              "title": "Discussion Post",
              "region": "us-east-1",
              "comments": [
                {
                  "id": "comment-1",
                  "content": "Great post!",
                  "region": "us-west-2",
                  "author": {
                    "firstName": "Jane",
                    "region": "us-west-2"
                  }
                },
                {
                  "id": "comment-2",
                  "content": "I agree!",
                  "region": "us-east-1",
                  "author": {
                    "firstName": "Bob",
                    "region": "us-east-1"
                  }
                }
              ]
            }
          }
        }
        "#);

        Ok(())
    }

    // Many-to-Many Relation Tests

    #[connector_test(schema(relations_schema))]
    async fn create_explicit_many_to_many_relation(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create post
        run_query!(
            &runner,
            r#"mutation {
                createOnePost(data: {
                    id: "post-1"
                    title: "Tech Article"
                    content: "Latest in technology"
                    authorId: "user-1"
                    region: "us-east-1"
                }) { id }
            }"#
        );

        // Connect tags to post via junction table
        let result = run_query!(
            &runner,
            r#"mutation {
                createManyPostTag(data: [
                    {
                        id: "posttag-1"
                        postId: "post-1"
                        tagId: "tag-1"
                        region: "us-east-1"
                    },
                    {
                        id: "posttag-2"
                        postId: "post-1"
                        tagId: "tag-3"
                        region: "us-east-1"
                    }
                ]) {
                    count
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"createManyPostTag":{"count":2}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn query_explicit_many_to_many_relations(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create post with tags
        run_query!(
            &runner,
            r#"mutation {
                createOnePost(data: {
                    id: "post-1"
                    title: "Business Tech Article"
                    content: "Intersection of business and technology"
                    authorId: "user-1"
                    region: "us-east-1"
                    postTags: {
                        create: [
                            {
                                id: "posttag-1"
                                tagId: "tag-1"
                                region: "us-east-1"
                            },
                            {
                                id: "posttag-2"
                                tagId: "tag-2"
                                region: "us-east-1"
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // Query post with tags
        let result = run_query_pretty!(
            &runner,
            r#"query {
                findUniquePost(where: { id: "post-1" }) {
                    id
                    title
                    region
                    postTags {
                        region
                        tag {
                            name
                            color
                        }
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r##"
        {
          "data": {
            "findUniquePost": {
              "id": "post-1",
              "title": "Business Tech Article",
              "region": "us-east-1",
              "postTags": [
                {
                  "region": "us-east-1",
                  "tag": {
                    "name": "technology",
                    "color": "#0066CC"
                  }
                },
                {
                  "region": "us-east-1",
                  "tag": {
                    "name": "business",
                    "color": "#CC6600"
                  }
                }
              ]
            }
          }
        }
        "##);

        Ok(())
    }

    // Connect/Disconnect Operations

    #[connector_test(schema(relations_schema))]
    async fn connect_existing_relations(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Connect user-1 to a new profile
        let result = run_query!(
            &runner,
            r#"mutation {
                updateOneUser(
                    where: { id: "user-1" }
                    data: {
                        profile: {
                            create: {
                                id: "profile-new"
                                bio: "Full Stack Developer"
                                region: "us-east-1"
                            }
                        }
                    }
                ) {
                    id
                    profile {
                        bio
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"{"data":{"updateOneUser":{"id":"user-1","profile":{"bio":"Full Stack Developer","region":"us-east-1"}}}}"#);

        // Connect a new profile to user-2
        run_query!(
            &runner,
            r#"mutation {
                createOneUserProfile(data: {
                    id: "profile-1"
                    userId: "user-2"
                    bio: "Data Scientist"
                    region: "us-west-2"
                }) { id }
            }"#
        );

        // This will fail because user-2 already has a profile, and the old one can't be
        // disconnected without deleting it.
        assert_error!(
            &runner,
            r#"mutation {
                updateOneUser(
                    where: { id: "user-2" },
                    data: {
                        profile: {
                            create: {
                                id: "profile-2-new"
                                bio: "Full Stack Developer"
                                region: "us-west-2"
                            }
                        }
                    }
                ) {
                    id
                    profile { id }
                }
            }"#,
            2014
        );

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn disconnect_relations(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create user with profile
        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    createOneUser(data: {
                        id: "user-disconnect"
                        email: "disconnect@example.com"
                        firstName: "Disconnect"
                        lastName: "Test"
                        region: "us-east-1"
                        profile: {
                            create: {
                                id: "profile-disconnect"
                                bio: "To be disconnected"
                                region: "us-east-1"
                            }
                        }
                    }) {
                        id
                        profile {
                            id
                            bio
                        }
                    }
                }"#
            ),
            @r#"{"data":{"createOneUser":{"id":"user-disconnect","profile":{"id":"profile-disconnect","bio":"To be disconnected"}}}}"#
        );

        // Connect profile to another user.
        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"mutation {
                    updateOneUser(
                        where: { id: "user-1" }
                        data: {
                            profile: {
                                connect: { id: "profile-disconnect" }
                            }
                        }
                    ) {
                        id
                        profile {
                            bio
                        }
                    }
                }"#
            ),
            @r#"{"data":{"updateOneUser":{"id":"user-1","profile":{"bio":"To be disconnected"}}}}"#
        );

        // Verify it has been disconnected from the previous user
        insta::assert_snapshot!(
            run_query!(
                &runner,
                r#"query {
                    findUniqueUser(where: { id: "user-disconnect" }) {
                        id
                        profile {
                            id
                        }
                    }
                }"#
            ),
            @r#"{"data":{"findUniqueUser":{"id":"user-disconnect","profile":null}}}"#
        );

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn query_cross_shard_organization_structure(runner: Runner) -> TestResult<()> {
        // Setup from previous test
        run_query!(
            &runner,
            r#"mutation {
                createOneOrganization(data: {
                    id: "org-1"
                    name: "TechCorp"
                    region: "us-east-1"
                    departments: {
                        create: [
                            {
                                id: "dept-1"
                                name: "Engineering"
                                region: "us-east-1"
                            },
                            {
                                id: "dept-2"
                                name: "Sales"
                                region: "us-west-2"
                            }
                        ]
                    }
                    employees: {
                        create: [
                            {
                                id: "emp-1"
                                firstName: "Alice"
                                lastName: "Engineer"
                                email: "alice@techcorp.com"
                                departmentId: "dept-1"
                                region: "us-east-1"
                            },
                            {
                                id: "emp-2"
                                firstName: "Bob"
                                lastName: "Sales"
                                email: "bob@techcorp.com"
                                departmentId: "dept-2"
                                region: "us-west-2"
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // Query organization with all related data across shards
        let result = run_query_pretty!(
            &runner,
            r#"query {
                findUniqueOrganization(where: { id: "org-1" }) {
                    id
                    name
                    region
                    departments {
                        id
                        name
                        region
                        employees {
                            firstName
                            lastName
                            region
                        }
                    }
                    employees {
                        id
                        firstName
                        region
                        department {
                            name
                            region
                        }
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "findUniqueOrganization": {
              "id": "org-1",
              "name": "TechCorp",
              "region": "us-east-1",
              "departments": [
                {
                  "id": "dept-1",
                  "name": "Engineering",
                  "region": "us-east-1",
                  "employees": [
                    {
                      "firstName": "Alice",
                      "lastName": "Engineer",
                      "region": "us-east-1"
                    }
                  ]
                },
                {
                  "id": "dept-2",
                  "name": "Sales",
                  "region": "us-west-2",
                  "employees": [
                    {
                      "firstName": "Bob",
                      "lastName": "Sales",
                      "region": "us-west-2"
                    }
                  ]
                }
              ],
              "employees": [
                {
                  "id": "emp-1",
                  "firstName": "Alice",
                  "region": "us-east-1",
                  "department": {
                    "name": "Engineering",
                    "region": "us-east-1"
                  }
                },
                {
                  "id": "emp-2",
                  "firstName": "Bob",
                  "region": "us-west-2",
                  "department": {
                    "name": "Sales",
                    "region": "us-west-2"
                  }
                }
              ]
            }
          }
        }
        "#);

        Ok(())
    }

    // Composite Shard Key Relations

    #[connector_test(schema(relations_schema))]
    async fn composite_shard_key_relations(runner: Runner) -> TestResult<()> {
        // Create tenant with composite shard key
        let result = run_query_pretty!(
            &runner,
            r#"mutation {
                createOneTenant(data: {
                    id: "tenant-1"
                    name: "Enterprise Customer"
                    region: "us-east-1"
                    tier: "premium"
                    projects: {
                        create: [
                            {
                                id: "project-1"
                                name: "Web Platform"
                                region: "us-east-1"
                                category: "web"
                                tasks: {
                                    create: [
                                        {
                                            id: "task-1"
                                            title: "Setup Database"
                                            region: "us-east-1"
                                            priority: "high"
                                        },
                                        {
                                            id: "task-2"
                                            title: "Design UI"
                                            region: "us-east-1"
                                            priority: "medium"
                                        }
                                    ]
                                }
                            },
                            {
                                id: "project-2"
                                name: "Mobile App"
                                region: "us-east-1"
                                category: "mobile"
                                tasks: {
                                    create: [
                                        {
                                            id: "task-3"
                                            title: "Setup CI/CD"
                                            region: "us-east-1"
                                            priority: "high"
                                        }
                                    ]
                                }
                            }
                        ]
                    }
                }) {
                    id
                    name
                    region
                    tier
                    projects {
                        id
                        name
                        category
                        region
                        tasks {
                            title
                            priority
                            region
                        }
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "createOneTenant": {
              "id": "tenant-1",
              "name": "Enterprise Customer",
              "region": "us-east-1",
              "tier": "premium",
              "projects": [
                {
                  "id": "project-1",
                  "name": "Web Platform",
                  "category": "web",
                  "region": "us-east-1",
                  "tasks": [
                    {
                      "title": "Setup Database",
                      "priority": "high",
                      "region": "us-east-1"
                    },
                    {
                      "title": "Design UI",
                      "priority": "medium",
                      "region": "us-east-1"
                    }
                  ]
                },
                {
                  "id": "project-2",
                  "name": "Mobile App",
                  "category": "mobile",
                  "region": "us-east-1",
                  "tasks": [
                    {
                      "title": "Setup CI/CD",
                      "priority": "high",
                      "region": "us-east-1"
                    }
                  ]
                }
              ]
            }
          }
        }
        "#);

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn query_composite_shard_hierarchy(runner: Runner) -> TestResult<()> {
        // Setup hierarchical data with composite shard keys
        run_query!(
            &runner,
            r#"mutation {
                createManyTenant(data: [
                    {
                        id: "tenant-1"
                        name: "StartupCorp"
                        region: "us-west-2"
                        tier: "basic"
                    },
                    {
                        id: "tenant-2"
                        name: "EnterpriseCorp"
                        region: "us-east-1"
                        tier: "premium"
                    }
                ]) { count }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
                createManyProject(data: [
                    {
                        id: "project-1"
                        name: "Quick Website"
                        tenantId: "tenant-1"
                        region: "us-west-2"
                        category: "web"
                    },
                    {
                        id: "project-2"
                        name: "Enterprise Platform"
                        tenantId: "tenant-2"
                        region: "us-east-1"
                        category: "platform"
                    }
                ]) { count }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
                createManyTask(data: [
                    {
                        id: "task-1"
                        title: "Create Landing Page"
                        projectId: "project-1"
                        region: "us-west-2"
                        priority: "low"
                    },
                    {
                        id: "task-2"
                        title: "Implement Authentication"
                        projectId: "project-2"
                        region: "us-east-1"
                        priority: "critical"
                    }
                ]) { count }
            }"#
        );

        // Query across the hierarchy with composite shard keys
        let result = run_query_pretty!(
            &runner,
            r#"query {
                findManyTenant(
                    where: {
                        region: "us-east-1"
                        tier: "premium"
                    }
                ) {
                    id
                    name
                    region
                    tier
                    projects(
                        where: {
                            region: "us-east-1"
                        }
                    ) {
                        name
                        category
                        region
                        tasks(
                            where: {
                                region: "us-east-1"
                                priority: "critical"
                            }
                        ) {
                            title
                            priority
                            region
                        }
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "findManyTenant": [
              {
                "id": "tenant-2",
                "name": "EnterpriseCorp",
                "region": "us-east-1",
                "tier": "premium",
                "projects": [
                  {
                    "name": "Enterprise Platform",
                    "category": "platform",
                    "region": "us-east-1",
                    "tasks": [
                      {
                        "title": "Implement Authentication",
                        "priority": "critical",
                        "region": "us-east-1"
                      }
                    ]
                  }
                ]
              }
            ]
          }
        }
        "#);

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn delete_relations_across_shards(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create nested structure
        run_query!(
            &runner,
            r#"mutation {
                createOnePost(data: {
                    id: "post-cascade"
                    title: "Post to be deleted"
                    content: "This will cascade"
                    authorId: "user-1"
                    region: "us-east-1"
                    comments: {
                        create: [
                            {
                                id: "comment-cascade-1"
                                content: "First comment"
                                authorId: "user-2"
                                region: "us-west-2"
                            },
                            {
                                id: "comment-cascade-2"
                                content: "Second comment"
                                authorId: "user-3"
                                region: "us-east-1"
                            }
                        ]
                    }
                    postTags: {
                        create: [
                            {
                                id: "posttag-cascade"
                                tagId: "tag-1"
                                region: "us-east-1"
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // Try to delete the post
        assert_error!(
            &runner,
            r#"mutation {
                deleteOnePost(where: { id: "post-cascade" }) {
                    id
                    title
                }
            }"#,
            2014
        );

        // Delete related data
        run_query!(
            &runner,
            r#"mutation {
                deleteManyComment(where: { postId: "post-cascade" }) {
                    count
                }
            }"#
        );
        run_query!(
            &runner,
            r#"mutation {
                deleteManyPostTag(where: { postId: "post-cascade" }) {
                    count
                }
            }"#
        );

        // Then delete the post
        let result = run_query!(
            &runner,
            r#"mutation {
                deleteOnePost(where: { id: "post-cascade" }) {
                    id
                    title
                }
            }"#
        );

        insta::assert_snapshot!(
            result,
            @r###"{"data":{"deleteOnePost":{"id":"post-cascade","title":"Post to be deleted"}}}"###
        );

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn relation_filters_across_shards(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create posts and comments across shards
        run_query!(
            &runner,
            r#"mutation {
                createManyPost(data: [
                    {
                        id: "filter-post-1"
                        title: "Popular Post"
                        content: "This will have many comments"
                        authorId: "user-1"
                        published: true
                        region: "us-east-1"
                    },
                    {
                        id: "filter-post-2"
                        title: "Unpopular Post"
                        content: "This will have no comments"
                        authorId: "user-2"
                        published: true
                        region: "us-west-2"
                    }
                ]) { count }
            }"#
        );

        run_query!(
            &runner,
            r#"mutation {
                createManyComment(data: [
                    {
                        id: "filter-comment-1"
                        content: "Great post!"
                        authorId: "user-2"
                        postId: "filter-post-1"
                        region: "us-west-2"
                    },
                    {
                        id: "filter-comment-2"
                        content: "Amazing content!"
                        authorId: "user-3"
                        postId: "filter-post-1"
                        region: "us-east-1"
                    }
                ]) { count }
            }"#
        );

        // Find posts that have comments (some relation filter)
        let result = run_query_pretty!(
            &runner,
            r#"query {
                findManyPost(
                    where: {
                        comments: {
                            some: {
                                content: { contains: "Great" }
                            }
                        }
                    }
                ) {
                    id
                    title
                    region
                    comments {
                        content
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "findManyPost": [
              {
                "id": "filter-post-1",
                "title": "Popular Post",
                "region": "us-east-1",
                "comments": [
                  {
                    "content": "Great post!",
                    "region": "us-west-2"
                  },
                  {
                    "content": "Amazing content!",
                    "region": "us-east-1"
                  }
                ]
              }
            ]
          }
        }
        "#);

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn nested_writes_across_shards(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        run_query!(
            &runner,
            r#"mutation {
                createOnePost(data: {
                    id: "post-1"
                    authorId: "user-1"
                    title: "Initial Post"
                    content: "Initial post content"
                    region: "us-east-1"
                    comments: {
                        create: [
                            {
                                id: "comment-1"
                                content: "Great post!"
                                authorId: "user-2"
                                region: "us-east-1"
                            }
                        ]
                    }
                }) {
                    id
                }
            }"#
        );

        // Complex nested write operation
        let result = run_query_pretty!(
            &runner,
            r#"mutation {
                updateOneUser(
                    where: { id: "user-1" }
                    data: {
                        posts: {
                            create: [
                                {
                                    id: "nested-post-1"
                                    title: "Nested Created Post"
                                    content: "Created via nested write"
                                    region: "us-east-1"
                                    comments: {
                                        create: [
                                            {
                                                id: "nested-comment-1"
                                                content: "Deeply nested comment"
                                                authorId: "user-2"
                                                region: "us-west-2"
                                            }
                                        ]
                                    }
                                }
                            ]
                            update: {
                                where: { id: "post-1" }
                                data: { title: "Updated via nested write" }
                            }
                        }
                        profile: {
                            upsert: {
                                create: {
                                    id: "upserted-profile"
                                    bio: "Created via upsert"
                                    region: "us-east-1"
                                }
                                update: {
                                    bio: "Updated via upsert"
                                }
                            }
                        }
                    }
                ) {
                    id
                    posts {
                        id
                        title
                        region
                        comments {
                            content
                            region
                        }
                    }
                    profile {
                        bio
                        region
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "updateOneUser": {
              "id": "user-1",
              "posts": [
                {
                  "id": "nested-post-1",
                  "title": "Nested Created Post",
                  "region": "us-east-1",
                  "comments": [
                    {
                      "content": "Deeply nested comment",
                      "region": "us-west-2"
                    }
                  ]
                },
                {
                  "id": "post-1",
                  "title": "Updated via nested write",
                  "region": "us-east-1",
                  "comments": [
                    {
                      "content": "Great post!",
                      "region": "us-east-1"
                    }
                  ]
                }
              ],
              "profile": {
                "bio": "Created via upsert",
                "region": "us-east-1"
              }
            }
          }
        }
        "#);

        Ok(())
    }

    #[connector_test(schema(relations_schema))]
    async fn relation_count_and_aggregates(runner: Runner) -> TestResult<()> {
        setup_relation_test_data(&runner).await?;

        // Create posts with different numbers of comments
        run_query!(
            &runner,
            r#"mutation {
                createOnePost(data: {
                    id: "count-post-1"
                    title: "Post with Many Comments"
                    content: "Popular post"
                    authorId: "user-1"
                    region: "us-east-1"
                    comments: {
                        create: [
                            {
                                id: "count-comment-1"
                                content: "First comment"
                                authorId: "user-2"
                                region: "us-west-2"
                            },
                            {
                                id: "count-comment-2"
                                content: "Second comment"
                                authorId: "user-3"
                                region: "us-east-1"
                            },
                            {
                                id: "count-comment-3"
                                content: "Third comment"
                                authorId: "user-2"
                                region: "us-west-2"
                            }
                        ]
                    }
                }) { id }
            }"#
        );

        // Query with relation counts
        let result = run_query_pretty!(
            &runner,
            r#"query {
                findManyUser {
                    id
                    firstName
                    region
                    _count {
                        posts
                        authoredComments
                    }
                    posts {
                        id
                        title
                        _count {
                            comments
                        }
                    }
                }
            }"#
        );

        insta::assert_snapshot!(result, @r#"
        {
          "data": {
            "findManyUser": [
              {
                "id": "user-1",
                "firstName": "John",
                "region": "us-east-1",
                "_count": {
                  "posts": 1,
                  "authoredComments": 0
                },
                "posts": [
                  {
                    "id": "count-post-1",
                    "title": "Post with Many Comments",
                    "_count": {
                      "comments": 3
                    }
                  }
                ]
              },
              {
                "id": "user-2",
                "firstName": "Jane",
                "region": "us-west-2",
                "_count": {
                  "posts": 0,
                  "authoredComments": 2
                },
                "posts": []
              },
              {
                "id": "user-3",
                "firstName": "Bob",
                "region": "us-east-1",
                "_count": {
                  "posts": 0,
                  "authoredComments": 1
                },
                "posts": []
              }
            ]
          }
        }
        "#);

        Ok(())
    }
}
