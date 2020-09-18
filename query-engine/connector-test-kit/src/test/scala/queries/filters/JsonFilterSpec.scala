package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.PostgresConnectorTag
import util._

class JsonFilterSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(PostgresConnectorTag)

  val project = SchemaDsl.fromStringV11() {
    s"""model Model {
       |   id   Int   @id
       |   json Json?
       |}"""
  }

  override def beforeEach(): Unit = {
    database.setup(project)

    super.beforeEach()
  }

  "Using a Json field in where clause" should "work" in {
    create(1, Some("{}"))
    create(2, Some("""{\"a\":\"b\"}"""))
    create(3, None)

    server
      .query("""query { findManyModel(where: { json: { equals: "{}" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1}]}}""")

    server
      .query("""query { findManyModel(where: { json: { not: "{}" }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":2}]}}""")

    server
      .query("""query { findManyModel(where: { json: { not: null }}) { id }}""", project, legacy = false)
      .toString should be("""{"data":{"findManyModel":[{"id":1},{"id":2}]}}""")
  }

  "A Json field in where clause" should "not have a shorthands for equals" in {
    server
      .queryThatMustFail(
        """query { findManyModel(where: { json: "{}" }) { id }}""",
        project,
        errorCode = 2009,
        errorContains = """`Value types mismatch. Have: String(\"{}\"), want: Object(JsonNullableFilter)` at `Query.findManyModel.where.ModelWhereInput.json`""",
        legacy = false
      )

    server
      .queryThatMustFail(
        """query { findManyModel(where: { json: null }) { id }}""",
        project,
        errorCode = 2012,
        errorContains = """Missing a required value at `Query.findManyModel.where.ModelWhereInput.json`""",
        legacy = false
      )
  }

  "A Json field in where clause" should "only have shorthands for nested not" in {
    server
      .queryThatMustFail(
        """query { findManyModel(where: { json: { not: { equals: "{}" }}}) { id }}""",
        project,
        errorCode = 2009,
        errorContains =
          """`Query.findManyModel.where.ModelWhereInput.json.JsonNullableFilter.not`: Value types mismatch. Have: Object({\"equals\": String(\"{}\")}), want: Json""",
        legacy = false
      )

    server
      .queryThatMustFail(
        """query { findManyModel(where: { json: { not: { equals: null }}}) { id }}""",
        project,
        errorCode = 2009,
        errorContains =
          """`Query.findManyModel.where.ModelWhereInput.json.JsonNullableFilter.not`: Value types mismatch. Have: Object({\"equals\": Null}), want: Json""",
        legacy = false
      )

  }

  def create(id: Int, json: Option[String]): Unit = {
    val j = json match {
      case Some(x) => s""""$x""""
      case None    => "null"
    }

    server.query(s"""mutation { createOneModel(data: { id: $id, json: $j }) { id }}""", project, legacy = false)
  }
}
