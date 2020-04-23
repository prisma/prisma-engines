package writes.dataTypes.json

import org.scalatest.{FlatSpec, Matchers}
import util._

class JsonSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Using a json field" should "work" ignore {
    val project = ProjectDsl.fromString {
      """|model Model {
         | id   String @id @default(cuid())
         | field Json
         |}"""
    }

    database.setup(project)

    val res = server.query(
      s"""
         |mutation {
         |  createModel(
         |    data: {
         |      field: "{\\"a\\": \\"b\\" }"
         |    }
         |  ) {
         |    field 
         |  }
         |}""".stripMargin,
      project
    )

    res.toString should be("""{"data":{"createModel":{"field":"{\"a\":\"b\"}"}}}""")
  }
}
