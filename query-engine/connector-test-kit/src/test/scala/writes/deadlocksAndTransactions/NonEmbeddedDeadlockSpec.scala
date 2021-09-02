package writes.deadlocksAndTransactions

import org.scalatest.time.{Seconds, Span}
import org.scalatest.{FlatSpec, Matchers, Retries}
import util.ConnectorCapability.{JoinRelationLinksCapability, ScalarListsCapability}
import util._

// RS: Won't port, this is a load test.
class NonEmbeddedDeadlockSpec extends FlatSpec with Matchers with Retries with ApiSpecBase with AwaitUtils {

  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability, ScalarListsCapability)
  override def doNotRunForConnectors  = Set(ConnectorTag.SQLiteConnectorTag)

  override def withFixture(test: NoArgTest) = {
    val delay = Span(5, Seconds) // we assume that the process gets overwhelmed sometimes by the concurrent requests. Give it a bit of time to recover before retrying.
    withRetry(delay) {
      super.withFixture(test)
    }
  }

  val fifty = Vector.range(0, 50)

  "updating single item many times" should "not cause deadlocks" in {
    testDataModels.testV11 { project =>
      val createResult = server.query(
        """mutation {
        |  createTodo(
        |    data: {
        |      comments: {
        |        create: [{text: "comment1"}, {text: "comment2"}]
        |      }
        |    }
        |  ){
        |    id
        |    comments { id }
        |  }
        |}""",
        project
      )

      val todoId = createResult.pathAsString("data.createTodo.id")

      def exec(i: Int) =
        server.query(
          s"""mutation {
           |  updateTodo(
           |    where: { id: "$todoId" }
           |    data:{
           |      a: { set: "$i" }
           |    }
           |  ){
           |    a
           |  }
           |}
      """,
          project
        )

      fifty.par.foreach(i => exec(i))
    }
  }

  "updating single item many times with scalar list values" should "not cause deadlocks" in {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Todo {
        |   id   String @id @default(cuid())
        |   a    String?
        |   tags String[]
        |}
        """
    }
    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createTodo(
        |    data: {}
        |  ){
        |    id
        |  }
        |}""",
      project
    )

    val todoId = createResult.pathAsString("data.createTodo.id")

    def exec(i: Int) =
      server.query(
        s"""mutation {
           |  updateTodo(
           |    where: { id: "$todoId" }
           |    data:{
           |      a: { set: "$i" }
           |      tags: {
           |        set: ["important", "doitnow"]
           |      }
           |    }
           |  ){
           |    a
           |  }
           |}
      """,
        project
      )

    fifty.par.foreach(i => exec(i))

  }

  "updating single item and relations many times" should "not cause deadlocks" in {
    testDataModels.testV11 { project =>
      val createResult = server.query(
        """mutation {
        |  createTodo(
        |    data: {
        |      comments: {
        |        create: [{text: "comment1"}, {text: "comment2"}]
        |      }
        |    }
        |  ){
        |    id
        |    comments { id }
        |  }
        |}""",
        project
      )

      val todoId     = createResult.pathAsString("data.createTodo.id")
      val comment1Id = createResult.pathAsString("data.createTodo.comments.[0].id")
      val comment2Id = createResult.pathAsString("data.createTodo.comments.[1].id")

      def exec(i: Int) =
        server.query(
          s"""mutation {
           |  updateTodo(
           |    where: { id: "$todoId" }
           |    data:{
           |      a: { set: "$i" }
           |      comments: {
           |        update: [{where: {id: "$comment1Id"}, data: {text: { set: "update $i" }}}]
           |      }
           |    }
           |  ){
           |    a
           |  }
           |}
      """,
          project
        )

      fifty.par.foreach(i => exec(i))

    }
  }

  "creating many items with relations" should "not cause deadlocks" in {
    testDataModels.testV11 { project =>
      def exec(i: Int) =
        server.query(
          s"""mutation {
             |  createTodo(
             |    data:{
             |      a: "a",
             |      comments: {
             |        create: [
             |           {text: "first comment: $i"}
             |        ]
             |      }
             |    }
             |  ){
             |    a
             |  }
             |}
        """,
          project
        )

      fifty.par.foreach(i => exec(i))

    }
  }

  "deleting many items" should "not cause deadlocks" in {
    testDataModels.testV11 { project =>
      def create() =
        server.query(
          """mutation {
          |  createTodo(
          |    data: {
          |      a: "b",
          |      comments: {
          |        create: [{text: "comment1"}, {text: "comment2"}]
          |      }
          |    }
          |  ){
          |    id
          |    comments { id }
          |  }
          |}""",
          project
        )

      val todoIds = fifty.par.map(i => create().pathAsString("data.createTodo.id"))

      def exec(id: String) =
        server.query(
          s"""mutation {
           |  deleteTodo(
           |    where: { id: "$id" }
           |  ){
           |    a
           |  }
           |}
      """,
          project
        )

      todoIds.par.foreach(id => exec(id))
    }
  }

  val testDataModels = {
    // TODO: use new syntax for Mongo
    val dm1 = """
        model Todo {
           id       String    @id @default(cuid())
           a        String?
           comments Comment[] @relation(references: [id])
        }

        model Comment {
           id   String @id @default(cuid())
           text String?
           todo Todo?
        }
      """

    // TODO: use new syntax for Mongo
    val dm2 = """
        model Todo {
           id       String   @id @default(cuid())
           a        String?
           comments Comment[]
        }

        model Comment {
           id   String  @id @default(cuid())
           text String?
           todo Todo?   @relation(references: [id])
        }
      """

    val dm3 = """
        model Todo {
           id       String @id @default(cuid())
           a        String?
           comments Comment[]
        }

        model Comment {
           id     String  @id @default(cuid())
           text   String?
           todoId String?

           todo Todo? @relation(fields:[todoId], references: [id])
        }
      """

//    val dm4 = """
//        model Todo {
//           id       String @id @default(cuid())
//           a        String?
//           comments Comment[] @relation(link: TABLE)
//        }
//
//        model Comment {
//           id   String @id @default(cuid())
//           text String?
//           todo Todo?
//        }
//      """

    TestDataModels(mongo = Vector(dm1, dm2), sql = Vector(dm3))
  }
}
