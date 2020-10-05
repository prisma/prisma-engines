package writes.dataTypes.bytes

import org.scalatest.{FlatSpec, Matchers}
import util._

class BytesSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Using a bytes field" should "work" in {
    val project = ProjectDsl.fromString {
      """|model Model {
         | id    Int    @id
         | field Bytes? @default("dGVzdA==")
         |}"""
    }

    database.setup(project)

    var res = server.query(
      s"""
         |mutation {
         |  createOneModel(
         |    data: {
         |      id: 1
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"createOneModel":{"field":"dGVzdA=="}}}""")

    res = server.query(
      s"""
         |mutation {
         |  updateOneModel(
         |    where: { id: 1 }
         |    data: {
         |      field: "dA=="
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"updateOneModel":{"field":"dA=="}}}""")

    res = server.query(
      s"""
         |mutation {
         |  updateOneModel(
         |    where: { id: 1 }
         |    data: {
         |      field: null
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"updateOneModel":{"field":null}}}""")
  }
}
