package writes.dataTypes.bigint

import org.scalatest.{FlatSpec, Matchers}
import util._

class BigIntSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Using a BigInt field" should "work" in {
    val project = ProjectDsl.fromString {
      """|model Model {
         | id    Int    @id
         | field BigInt? @default(123456789012341234)
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

    res.toString should be("""{"data":{"createOneModel":{"field":"123456789012341234"}}}""")

    res = server.query(
      s"""
         |mutation {
         |  updateOneModel(
         |    where: { id: 1 }
         |    data: {
         |      field: "9223372036854775807"
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"updateOneModel":{"field":"9223372036854775807"}}}""")

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
