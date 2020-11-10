package writes.dataTypes.scalarLists

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.PostgresConnectorTag
import util._

class JsonListsSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {

  override def runOnlyForConnectors: Set[ConnectorTag] = Set(PostgresConnectorTag)

  val project = ProjectDsl.fromString {
    s"""
      |model ScalarModel {
      |  id    Int    @id
      |  json  Json[] @test.Json
      |  jsonB Json[] @test.JsonB
      |}
    """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "Json lists" should "behave like regular values for create and update operations" in {
    var res = server.query(
      s"""mutation {
         |  createOneScalarModel(data: {
         |    id: 1,
         |    json: { set: ["{\\"a\\":\\"b\\"}", "{\\"c\\":\\"d\\"}"] }
         |    jsonB: { set: ["{\\"a\\":\\"b\\"}", "{\\"c\\":\\"d\\"}"] }
         |  }) {
         |    json
         |    jsonB
         |  }
         |}""",
      project = project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneScalarModel":{"json":["{\"a\":\"b\"}","{\"c\":\"d\"}"],"jsonB":["{\"a\":\"b\"}","{\"c\":\"d\"}"]}}}""")

    res = server.query(
      s"""mutation {
         |  updateOneScalarModel(where: { id: 1 }, data: {
         |    json: { set: ["{\\"e\\":\\"f\\"}"] }
         |    jsonB: { set: ["{\\"e\\":\\"f\\"}"] }
         |  }) {
         |    json
         |    jsonB
         |  }
         |}""",
      project = project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateOneScalarModel":{"json":["{\"e\":\"f\"}"],"jsonB":["{\"e\":\"f\"}"]}}}""")
  }

  "Setting Json lists to empty" should "be possible" in {
    val res = server.query(
      s"""mutation {
         |  createOneScalarModel(data: {
         |    id: 1,
         |    json: { set: [] }
         |    jsonB: { set: [] }
         |  }) {
         |    json
         |    jsonB
         |  }
         |}""",
      project = project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneScalarModel":{"json":[],"jsonB":[]}}}""")
  }

  "Setting Json lists to wurst" should "be wurst" in {
    val res = server.query(
      s"""mutation {
         |  createOneScalarModel(data: {
         |    id: 1,
         |    json: { set: "[{\\"a\\":\\"b\\"}]" }
         |    jsonB: { set: "[{\\"a\\":\\"b\\"}]" }
         |  }) {
         |    json
         |    jsonB
         |  }
         |}""",
      project = project,
      legacy = false
    )

//    res.toString() should be("""{"data":{"createOneScalarModel":{"json":[],"jsonB":[]}}}""")
  }
}
