package writes.withSingleId.ids

import org.scalatest.{FlatSpec, Matchers}
import util._

class RequiredOwnIdSpec extends FlatSpec with Matchers with ApiSpecBase {
  val schema =
    """
    |model ScalarModel {
    |   id          String @id
    |   optString   String?
    |}""".stripMargin

  val project = ProjectDsl.fromString { schema }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "A Create Mutation" should "create and return item" in {
    val res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: "thisismyownid"
         |  }){id }
         |}""".stripMargin,
      project = project
    )

    res should be(
      s"""{"data":{"createScalarModel":{"id":"thisismyownid"}}}""".parseJson)
   }

  "A Create Mutation" should "error if a required id is not provided" in {
    server.queryThatMustFail(
      s"""mutation {
         |  createScalarModel(data: {
         |    optString: "iforgotmyid"
         |  }){id }
         |}""".stripMargin,
      project = project,0
    )
  }
}
