package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util._

class XmlFilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    s"""model Model {
       |   id  Int  @id
       |   xml Xml?
       |}"""
  }

  override def beforeEach(): Unit = {
    database.setup(project)
    super.beforeEach()
  }

  "Using a XML field in where clause" should "work" taggedAs (IgnoreMySql, IgnoreSQLite) in {
    create(1, Some("<horse>neigh</horse>"))
    create(2, Some("<pig>oink</pig>"))
    create(3, None)

    server
      .query("""query { findManyModel(where: { xml: { equals: "<horse>neigh</horse>" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { xml: { not: "<horse>neigh</horse>" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    server
      .query("""query { findManyModel(where: { xml: { not: null }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")
  }

  "A XML field in where clause" should "have shorthands" taggedAs (IgnoreMySql, IgnoreSQLite) in {
    create(1, Some("<horse>neigh</horse>"))
    create(2, Some("dA=="))
    create(3, None)

    server
      .query("""query { findManyModel(where: { xml: "<horse>neigh</horse>" }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { xml: null }) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":3}]}}""")
  }

  def create(id: Int, xml: Option[String]): Unit = {
    val x = xml match {
      case Some(x) => s""""$x""""
      case None    => "null"
    }

    server.query(s"""mutation { createOneModel(data: { id: $id, xml: $x }) { id }}""", project, legacy = false)
  }
}
