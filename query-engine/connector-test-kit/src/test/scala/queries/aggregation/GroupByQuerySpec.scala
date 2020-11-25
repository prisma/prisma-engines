package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

class GroupByQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Model {
      |  id    String  @id @default(cuid())
      |  float Float   @map("db_float")
      |  int   Int     @map("db_int")
      |  dec   Decimal @map("db_dec")
      |  s     String  @map("db_s")
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def create(float: Double, int: Int, dec: String, s: String, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createModel(data: { $idString float: $float, int: $int, dec: $dec, s: "$s" }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  "Using a simple groupBy without any records in the database" should "return no groups" in {
    val result = server.query(
      s"""{
         |  groupByModel(by: [id, float, int]) {
         |    count { id }
         |    float
         |    sum { int }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[]}}""")
  }

  "Using a simple groupBy" should "return the correct groups" in {
    create(10.1, 5, "1.1", "group1", Some("1"))
    create(5.5, 0, "6.7", "group1", Some("2"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "11", "group3", Some("4"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [s]) {
         |    count { s }
         |    sum { float }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[]}}""")
  }

  // todo
  // orderBy
  // where
  // skip
  // take
}
