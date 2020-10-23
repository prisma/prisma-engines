package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util._

class BytesFilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    s"""model Model {
       |   id   Int   @id
       |   bytes Bytes?
       |}"""
  }

  override def beforeEach(): Unit = {
    database.setup(project)
    super.beforeEach()
  }

  "Using a Bytes field in where clause" should "work" in {
    create(1, Some("dGVzdA=="))
    create(2, Some("dA=="))
    create(3, None)

    server
      .query("""query { findManyModel(where: { bytes: { equals: "dGVzdA==" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { bytes: { not: "dGVzdA==" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    server
      .query("""query { findManyModel(where: { bytes: { not: null }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")
  }

  "A Bytes field in where clause" should "have shorthands" in {
    create(1, Some("dGVzdA=="))
    create(2, Some("dA=="))
    create(3, None)

    server
      .query("""query { findManyModel(where: { bytes: "dGVzdA==" }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { bytes: null }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":3}]}}""")
  }

  def create(id: Int, bytes: Option[String]): Unit = {
    val b = bytes match {
      case Some(x) => s""""$x""""
      case None    => "null"
    }

    server.query(s"""mutation { createOneModel(data: { id: $id, bytes: $b }) { id }}""", project, legacy = false)
  }
}
