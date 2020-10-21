package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util._

class DecimalFilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    s"""model Model {
       |   id  Int      @id
       |   dec Decimal?
       |}"""
  }

  override def beforeEach(): Unit = {
    database.setup(project)
    super.beforeEach()
  }

  "Using a Decimal field in a basic (not) equals where clause" should "work" in {
    create(1, Some("5.5"))
    create(2, Some("1"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { dec: { equals: "5.5" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { dec: { not: "1.0" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { dec: { not: null }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")
  }

  "A Decimal field in where clause" should "have (not) equals shorthands" in {
    create(1, Some("5.5"))
    create(2, Some("1"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { dec: "5.5" }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { dec: null }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":3}]}}""")
  }

  "Using a Decimal field with inclusion filter clauses" should "work" in {
    create(1, Some("5.5"))
    create(2, Some("1"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { dec: { in: ["5.5", "1.0"] }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    server
      .query("""query { findManyModel(where: { dec: { notIn: ["1.0"] }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { dec: { not: { in: ["1.0"] }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")
  }

  "Using a Decimal field with numeric comparison filter clauses" should "work" in {
    create(1, Some("5.5"))
    create(2, Some("1"))
    create(3, None)

    // Gt
    server
      .query("""query { findManyModel(where: { dec: { gt: "1.0" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    // Not gt => lte
    server
      .query("""query { findManyModel(where: { dec: { not: { gt: "1.0" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    // Gte
    server
      .query("""query { findManyModel(where: { dec: { gte: "1.0" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    // Not gte => lt
    server
      .query("""query { findManyModel(where: { dec: { not: { gte: "5.5" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    // Lt
    server
      .query("""query { findManyModel(where: { dec: { lt: "6" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    // Not lt => gte
    server
      .query("""query { findManyModel(where: { dec: { not: { lt: "5.5" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    // Lte
    server
      .query("""query { findManyModel(where: { dec: { lte: "5.5" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    // Not lte => gt
    server
      .query("""query { findManyModel(where: { dec: { not: { lte: "1" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")
  }

  def create(id: Int, dec: Option[String]): Unit = {
    val d = dec match {
      case Some(x) => s""""$x""""
      case None    => "null"
    }

    server.query(s"""mutation { createOneModel(data: { id: $id, dec: $d }) { id }}""", project, legacy = false)
  }
}
