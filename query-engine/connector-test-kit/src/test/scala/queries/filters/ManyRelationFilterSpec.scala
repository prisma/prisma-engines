package queries.filters

import org.scalatest._
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class ManyRelationFilterSpec extends FlatSpec with Matchers with ApiSpecBase {

  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val project = ProjectDsl.fromString {
    """
      |model Blog {
      |   id    String @id @default(cuid())
      |   name  String
      |   posts Post[]
      |}
      |
      |model Post {
      |   id         String @id @default(cuid())
      |   title      String
      |   popularity Int
      |   blog_id    String
      |   comments   Comment[]
      |   
      |   blog Blog @relation(fields: [blog_id], references: [id])
      |   
      |   @@index([blog_id])
      |}
      |
      |model Comment {
      |   id      String @id @default(cuid())
      |   text    String
      |   likes   Int
      |   post_id String
      |
      |   post Post @relation(fields: [post_id], references: [id])
      |
      |   @@index([post_id])
      |}
    """
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach() = {
    super.beforeEach()
    database.truncateProjectTables(project)

    // add data
    server.query(
      """mutation {createBlog(
        |     data: {
        |       name: "blog 1",
        |       posts:{
        |         create:[
        |           {title: "post 1", popularity: 10, comments:{
        |                                               create: [{text:"comment 1", likes: 0 },
        |                                                        {text:"comment 2", likes: 5},
        |                                                        {text:"comment 3", likes: 10}]
        |                                             }
        |           },
        |           {title: "post 2", popularity: 2,  comments:{
        |                                               create: [{text:"comment 4", likes: 10}]
        |                                             }
        |           }
        |         ]
        |      }
        | }
        |){name}}""".stripMargin,
      project = project
    )
    server.query(
      """mutation {createBlog(data:{
        |                         name: "blog 2",
        |                         posts: {create: [
        |                                   {title: "post 3",
        |                                    popularity: 1000,
        |                                    comments:{create: [
        |                                             {text:"comment 5", likes: 1000}
        |                                             ]}
        |                                             }]}
        |                                             }){name}}""".stripMargin,
      project = project
    )
  }

  "simple scalar filter" should "work" in {
    server.query(query = """{blogs{posts(where: { popularity: { gte: 5 }}, orderBy: { id: asc }){title}}}""", project = project).toString should be(
      """{"data":{"blogs":[{"posts":[{"title":"post 1"}]},{"posts":[{"title":"post 3"}]}]}}""")
  }

  "1 level 1-relation filter" should "work" in {
    server.query(query = """{posts(where:{ blog: { is: { name: { equals: "blog 1"}}}}, orderBy: { id: asc }){title}}""", project = project).toString should be(
      """{"data":{"posts":[{"title":"post 1"},{"title":"post 2"}]}}""")
  }

  "1 level m-relation filter" should "work for `some`" in {
    server.query(query = """{ blogs(where: { posts: { some: { popularity: { gte: 5 }}}}, orderBy: { id: asc }){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"},{"name":"blog 2"}]}}""")

    server.query(query = """{blogs(where:{ posts: { some: { popularity: { gte: 50 }}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 2"}]}}""")

    server
      .query(query = """{blogs(where:{ posts: { some:{ AND: [{title: { equals: "post 1" }}, { title: { equals: "post 2" }}]}}}){name}}""", project = project)
      .toString should be("""{"data":{"blogs":[]}}""")

    server
      .query(
        query = """
            |{
            |  blogs(
            |    where: {
            |      AND: [
            |        { posts: { some: { title: { equals: "post 1" } } } }
            |        { posts: { some: { title: { equals: "post 2" } } } }
            |      ]
            |    }
            |  ) {
            |    name
            |  }
            |}
          """.stripMargin,
        project = project
      )
      .toString should be("""{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server
      .query(
        query = """
          |{
          |  blogs(
          |    where: {
          |      posts: {
          |        some: {
          |          AND: [{ title: { equals: "post 1" } }, { popularity: { gte: 2 } }]
          |        }
          |      }
          |    }
          |  ) {
          |    name
          |  }
          |}
        """.stripMargin,
        project = project
      )
      .toString should be("""{"data":{"blogs":[{"name":"blog 1"}]}}""")
  }

  "1 level m-relation filter" should "work for `every`" taggedAs (IgnoreMongo) in {
    server.query(query = """{blogs(where: { posts: { every: { popularity: { gte: 2 }}}}, orderBy: { id: asc }){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"},{"name":"blog 2"}]}}""")

    server.query(query = """{blogs(where: { posts: { every: { popularity: { gte: 3 }}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 2"}]}}""")
  }

  "1 level m-relation filter" should "work for `none`" taggedAs (IgnoreMongo) in {
    server.query(query = """{blogs(where:{posts: { none:{popularity: { gte: 50 }}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server.query(query = """{blogs(where:{posts: { none:{popularity: { gte: 5 }}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")
  }

  "2 level m-relation filter" should "work for some/some" in {
    // some|some
    server.query(query = """{blogs(where:{posts: { some:{ comments: { some: { likes: { equals: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server.query(query = """{blogs(where: { posts: { some: { comments: { some: { likes: { equals: 1 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")
  }

  "2 level m-relation filter" should "work for `every`, `some` and `none`" taggedAs (IgnoreMongo) in {
    // some|every
    server
      .query(query = """{ blogs(where: { posts: { some: { comments: { every: { likes: { gte: 0 }}}}}}, orderBy: { id: asc }){name}}""", project = project)
      .toString should be("""{"data":{"blogs":[{"name":"blog 1"},{"name":"blog 2"}]}}""")

    server.query(query = """{blogs(where: { posts: { some: { comments: { every: { likes: { equals: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")

    // some|none
    server
      .query(query = """{ blogs(where: { posts: { some: { comments: { none: { likes: { equals: 0 }}}}}}, orderBy: { id: asc }){name}}""", project = project)
      .toString should be("""{"data":{"blogs":[{"name":"blog 1"},{"name":"blog 2"}]}}""")

    server.query(query = """{ blogs(where: { posts: { some: { comments: { none: { likes: { gte: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")

    // every|some
    server.query(query = """{ blogs(where: { posts: { every: { comments: { some: { likes: { equals: 10 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server.query(query = """{ blogs(where: { posts: { every: { comments: { some: { likes: { equals: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")

    // every|every
    server
      .query(query = """{ blogs(where: { posts: { every: { comments: { every: { likes: { gte: 0 }}}}}}, orderBy: { id: asc }){name}}""", project = project)
      .toString should be("""{"data":{"blogs":[{"name":"blog 1"},{"name":"blog 2"}]}}""")

    server.query(query = """{ blogs(where: { posts: { every: { comments: { every: { likes: { equals: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")

    // every|none
    server.query(query = """{ blogs(where:{posts: { every: { comments: { none: { likes: { gte: 100 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server.query(query = """{ blogs(where:{posts: { every: { comments: { none: { likes: { equals: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 2"}]}}""")

    // none|some
    server.query(query = """{ blogs(where: { posts: { none: { comments: { some: { likes: { gte: 100 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server.query(query = """{ blogs(where: { posts: { none: { comments: { some: { likes: { equals: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 2"}]}}""")

    // none|every
    server.query(query = """{ blogs(where: { posts: { none: { comments: { every: { likes: { gte: 11 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 1"}]}}""")

    server.query(query = """{ blogs(where: { posts: { none: { comments: { every: { likes: { gte: 0 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[]}}""")

    // none|none
    server
      .query(query = """{ blogs(where: { posts: { none: { comments: { none: { likes: { gte: 0 }}}}}}, orderBy: { id: asc }){name}}""", project = project)
      .toString should be("""{"data":{"blogs":[{"name":"blog 1"},{"name":"blog 2"}]}}""")

    server.query(query = """{ blogs(where: { posts: { none: { comments: { none: { likes: { gte: 11 }}}}}}){name}}""", project = project).toString should be(
      """{"data":{"blogs":[{"name":"blog 2"}]}}""")
  }

  "crazy filters" should "work" taggedAs (IgnoreMongo) in {
    server
      .query(
        query = """
            |{
            |  posts(
            |    where: {
            |      blog: {
            |        is: {
            |          posts: { some: { popularity: { gte: 5 } } }
            |          name: { contains: "Blog 1" }
            |        }
            |      }
            |      AND: [
            |        { comments: { none: { likes: { gte: 5 } } } },
            |        { comments: { some: { likes: { lte: 2 } } } }
            |      ]
            |    }
            |  ) {
            |    title
            |  }
            |}
          """.stripMargin,
        project = project
      )
      .toString should be("""{"data":{"posts":[]}}""")
  }

  "Join Relation Filter on many to many relation" should "work on one level" taggedAs (IgnorePostgres) in {
    val testDataModels = {
      val dm1 = """
        |model Post {
        |  id       String  @id @default(cuid())
        |  authors  AUser[] @relation(references: [id])
        |  title    String  @unique
        |}
        |
        |model AUser {
        |  id    String @id @default(cuid())
        |  name  String @unique
        |  posts Post[] @relation(references: [id])
        |}"""

      val dm2 = """
        |model Post {
        |  id      String  @id @default(cuid())
        |  authors AUser[] @relation(references: [id])
        |  title   String  @unique
        |}
        |
        |model AUser {
        |  id    String @id @default(cuid())
        |  name  String @unique
        |  posts Post[] @relation(references: [id])
        |}"""

      TestDataModels(mongo = Vector(dm1), sql = Vector(dm2))
    }

    testDataModels.testV11 { project =>
      server.query(s""" mutation {createPost(data: {title:"Title1"}) {title}} """, project)
      server.query(s""" mutation {createPost(data: {title:"Title2"}) {title}} """, project)
      server.query(s""" mutation {createAUser(data: {name:"Author1"}) {name}} """, project)
      server.query(s""" mutation {createAUser(data: {name:"Author2"}) {name}} """, project)

      server.query(s""" mutation {updateAUser(where: { name: "Author1"}, data:{posts:{connect:[{title: "Title1"},{title: "Title2"}]}}) {name}} """, project)
      server.query(s""" mutation {updateAUser(where: { name: "Author2"}, data:{posts:{connect:[{title: "Title1"},{title: "Title2"}]}}) {name}} """, project)

      server.query("""query{aUsers (orderBy: { id: asc }){name, posts(orderBy: { id: asc }){title}}}""", project).toString should be(
        """{"data":{"aUsers":[{"name":"Author1","posts":[{"title":"Title1"},{"title":"Title2"}]},{"name":"Author2","posts":[{"title":"Title1"},{"title":"Title2"}]}]}}""")

      server.query("""query{posts(orderBy: { id: asc }) {title, authors (orderBy: { id: asc }){name}}}""", project).toString should be(
        """{"data":{"posts":[{"title":"Title1","authors":[{"name":"Author1"},{"name":"Author2"}]},{"title":"Title2","authors":[{"name":"Author1"},{"name":"Author2"}]}]}}""")

      val res = server.query(
        """query{aUsers(where:{name: { startsWith: "Author2" }, posts: { some:{title: { endsWith: "1" }}}},orderBy: { id: asc }){name, posts(orderBy: { id: asc }){title}}}""",
        project
      )
      res.toString should be("""{"data":{"aUsers":[{"name":"Author2","posts":[{"title":"Title1"},{"title":"Title2"}]}]}}""")
    }

  }
}
