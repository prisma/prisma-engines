package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util._

class DefaultValueSpec extends FlatSpec with Matchers with ApiSpecBase {

  "A Create Mutation on a non-list field" should "utilize the defaultValue" in {
    val project = ProjectDsl.fromString {
      """
        |model ScalarModel {
        |  id        String  @id @default(cuid())
        |  reqString String? @default(value: "default")
        |}
      """
    }
    database.setup(project)

    val res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    }
         |  ){
         |  reqString
         |  }
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createScalarModel":{"reqString":"default"}}}""")

    val queryRes = server.query("""{ scalarModels{reqString}}""", project = project)

    queryRes.toString should be(s"""{"data":{"scalarModels":[{"reqString":"default"}]}}""")
  }

  "The default value" should "work for int" in {
    val project = ProjectDsl.fromString {
      """
        |model Service {
        |  id   String @id @default(cuid())
        |  name String
        |  int  Int?   @default(value: 1)
        |}
      """
    }
    database.setup(project)

    val res = server.query(
      s"""mutation createService{
         |  createService(
         |    data:{
         |      name: "issue1820"
         |    }
         |  ){
         |    name
         |    int
         |  }
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createService":{"name":"issue1820","int":1}}}""")
  }

  "The default value" should "work for enums" in {
    val project = ProjectDsl.fromString {
      """
        |enum IsActive{
        |  Yes
        |  No
        |}
        |
        |model Service {
        |  id           String    @id @default(cuid())
        |  name         String
        |  description  String?
        |  unit         String?
        |  active       IsActive? @default(value: Yes)
        |}
      """
    }
    database.setup(project)

    val res = server.query(
      s"""mutation createService{
         |  createService(
         |    data:{
         |      name: "issue1820"
         |    }
         |  ){
         |    name
         |    active
         |  }
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createService":{"name":"issue1820","active":"Yes"}}}""")
  }

  "The default value for updatedAt and createdAt" should "not be set if specific values are passed on create" in {
    val project = ProjectDsl.fromString {
      """
        |model User {
        |  id        String   @id @default(cuid())
        |  name      String
        |  createdAt DateTime @default(now())
        |  updatedAt DateTime @updatedAt
        |}
      """
    }
    database.setup(project)

    val res = server.query(
      s"""mutation {
         |  createUser(
         |    data:{
         |      name: "Just Bob"
         |      createdAt: "2000-01-01T00:00:00Z"
         |      updatedAt: "2001-01-01T00:00:00Z"
         |    }
         |  ){
         |    createdAt
         |    updatedAt
         |  }
         |}""",
      project = project
    )

    // We currently have a datetime precision of 3, so Prisma will add .000
    res.pathAsString("data.createUser.createdAt") should be("2000-01-01T00:00:00.000Z")
    res.pathAsString("data.createUser.updatedAt") should be("2001-01-01T00:00:00.000Z")
  }

  "Remapped enum default values" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model User {
        |  id        String   @id @default(cuid())
        |  name      Names    @default(Spiderman) @unique
        |  age       Int
        }
        |
        |enum Names {
        |   Spiderman @map("Peter Parker")
        |   Superman  @map("Clark Kent")
        |}
        |
      """
    }
    database.setup(project)

    val res = server.query(
      s"""mutation {
         |  createUser(
         |    data:{
         |      age: 21
         |    }
         |  ){
         |    name
         |  }
         |}""",
      project = project
    )

    res.toString() should be("{\"data\":{\"createUser\":{\"name\":\"Spiderman\"}}}")

    val res2 = server.query(
      s"""mutation {
         |  createUser(
         |    data:{
         |      name: Superman
         |      age: 32
         |    }
         |  ){
         |    name
         |  }
         |}""",
      project = project
    )

    res2.toString() should be("{\"data\":{\"createUser\":{\"name\":\"Superman\"}}}")

    val res3 = server.query(
      s"""query {
         |  user(
         |    where:{
         |      name: Superman
         |      }
         |  ){
         |    name,
         |    age
         |  }
         |}""",
      project = project
    )

    res3.toString() should be("{\"data\":{\"user\":{\"name\":\"Superman\",\"age\":32}}}")

    val res4 = server.query(
      s"""query {
         |  users(
         |    where:{
         |      name_in: [Spiderman, Superman]
         |      }
         |    orderBy: age_ASC
         |  ){
         |    name,
         |    age
         |  }
         |}""",
      project = project
    )

    res4.toString() should be("{\"data\":{\"users\":[{\"name\":\"Spiderman\",\"age\":21},{\"name\":\"Superman\",\"age\":32}]}}")

  }
}
