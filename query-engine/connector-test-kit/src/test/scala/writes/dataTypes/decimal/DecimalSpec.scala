package writes.dataTypes.decimal

import org.scalatest.{FlatSpec, Matchers}
import util._

// Ignored for MSSQL and SQLite because of low precision issues.
class DecimalSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Using a Decimal field" should "work" taggedAs (IgnoreSQLite, IgnoreMsSql) in {
    val project = ProjectDsl.fromString {
      """|model Model {
         | id    Int      @id
         | field Decimal? @default("1.00112233445566778899")
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

    res.toString should be("""{"data":{"createOneModel":{"field":"1.00112233445566778899"}}}""")

    res = server.query(
      s"""
         |mutation {
         |  updateOneModel(
         |    where: { id: 1 }
         |    data: {
         |      field: "0.09988776655443322"
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"updateOneModel":{"field":"0.09988776655443322"}}}""")

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
