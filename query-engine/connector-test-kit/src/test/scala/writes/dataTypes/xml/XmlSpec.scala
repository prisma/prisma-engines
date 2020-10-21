package writes.dataTypes.xml

import org.scalatest.{FlatSpec, Matchers}
import util._

class XmlSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Using a XML field" should "work" taggedAs (IgnoreMySql, IgnoreSQLite) in {
    val project = ProjectDsl.fromString {
      """|model Model {
         | id    Int  @id
         | field Xml? @default("<sense>none</sense>")
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

    res.toString should be("""{"data":{"createOneModel":{"field":"<sense>none</sense>"}}}""")

    res = server.query(
      s"""
         |mutation {
         |  updateOneModel(
         |    where: { id: 1 }
         |    data: {
         |      field: "<sense>some</sense>"
         |    }
         |  ) {
         |    field
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    res.toString should be("""{"data":{"updateOneModel":{"field":"<sense>some</sense>"}}}""")

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
