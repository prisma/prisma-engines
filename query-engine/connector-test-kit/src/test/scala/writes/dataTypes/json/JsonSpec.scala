package writes.dataTypes.json

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class JsonSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Using a json field" should "work" taggedAs (IgnoreMySql, IgnoreSQLite) in {
    val project = ProjectDsl.fromString {
      """|model Model {
         | id    String @id
         | field Json
         | list  Json[]
         |}"""
    }

    database.setup(project)

    var res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      id: "A"
         |      field: "{\\"a\\": \\"b\\" }"
         |    }
         |  ) {
         |    field 
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"field":"{\"a\":\"b\"}"}}}""")

    res = server.query(
      s"""
         |mutation {
         |  updateOneModel(
         |    where: { id: "A" }
         |    data: {
         |      id: { set: "1" }
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"updateOneModel":{"field":"{\"a\":\"b\"}"}}}""")
  }
}
