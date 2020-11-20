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

  def createItem(float: Double, int: Int, dec: String, s: String, id: Option[String] = None) = {
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

  "Using a simple groupBy without any records in the database" should "return 0 for all aggregations" in {
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

//    result.toString should be("""""")
  }

}
