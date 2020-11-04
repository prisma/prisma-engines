package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util._

class BigIntFilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    s"""model Model {
       |   id   Int      @id
       |   bInt BigInt?
       |}"""
  }

  override def beforeEach(): Unit = {
    database.setup(project)
    super.beforeEach()
  }

  "Using a BigInt field in a basic (not) equals where clause" should "work" in {
    create(1, Some("5"))
    create(2, Some("1"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { bInt: { equals: "5" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { bInt: { not: "1" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { bInt: { not: null }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")
  }

  "A Decimal field in where clause" should "have (not) equals shorthands" in {
    create(1, Some("5"))
    create(2, Some("1"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { bInt: "5" }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { bInt: null }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":3}]}}""")
  }

  "Using a BigInt field with inclusion filter clauses" should "work" in {
    create(1, Some("5"))
    create(2, Some("1"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { bInt: { in: ["5", "1"] }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    server
      .query("""query { findManyModel(where: { bInt: { notIn: ["1"] }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { bInt: { not: { in: ["1"] }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")
  }

  "Using a BigInt field with numeric comparison filter clauses" should "work" in {
    create(1, Some("5"))
    create(2, Some("1"))
    create(3, None)

    // Gt
    server
      .query("""query { findManyModel(where: { bInt: { gt: "1" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    // Not gt => lte
    server
      .query("""query { findManyModel(where: { bInt: { not: { gt: "1" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    // Gte
    server
      .query("""query { findManyModel(where: { bInt: { gte: "1" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    // Not gte => lt
    server
      .query("""query { findManyModel(where: { bInt: { not: { gte: "5" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    // Lt
    server
      .query("""query { findManyModel(where: { bInt: { lt: "6" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    // Not lt => gte
    server
      .query("""query { findManyModel(where: { bInt: { not: { lt: "5" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    // Lte
    server
      .query("""query { findManyModel(where: { bInt: { lte: "5" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")

    // Not lte => gt
    server
      .query("""query { findManyModel(where: { bInt: { not: { lte: "1" }}}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")
  }

  def create(id: Int, bInt: Option[String]): Unit = {
    val b = bInt match {
      case Some(x) => s""""$x""""
      case None    => "null"
    }

    server.query(s"""mutation { createOneModel(data: { id: $id, bInt: $b }) { id }}""", project, legacy = false)
  }
}
