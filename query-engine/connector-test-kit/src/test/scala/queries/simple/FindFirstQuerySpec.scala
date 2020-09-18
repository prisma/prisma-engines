package queries.simple

import org.scalatest.{FlatSpec, Matchers}
import util._

class FindFirstQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model TestModel {
      |  id    Int     @id
      |  field String?
      |}
    """.stripMargin
  }

  override def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
    database.truncateProjectTables(project)
  }

  "Fetching a record with findFirst" should "fetch the first matching record" in {
    create(1, Some("test1"))
    create(2, Some("test2"))
    create(3)
    create(4)
    create(5, Some("test3"))

    findFirst("{ id: 1 }") should be(1)
    findFirst("{ field: { not: null } }") should be(1)
    findFirst("{ field: { not: null } }", orderBy = Some("{ id: desc }")) should be(5)
    findFirst("{ field: { not: null } }", cursor = Some("{ id: 1 }"), take = Some("1"), skip = Some("1"), orderBy = Some("{ id: asc }")) should be(2)
  }

  "FindOne record with no results" should "return null" in {
    val result = server.query(
      s"""{
         |  findFirstTestModel {
         |    id
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findFirstTestModel":null}}""")
  }

  def create(id: Int, field: Option[String] = None): Unit = {
    val fieldValue = field.map(f => s""", field: "$f"""").getOrElse("")

    server.query(
      s"""mutation {
        |  createOneTestModel(data: { id: $id $fieldValue }) {
        |    id
        |  }
        |}""".stripMargin,
      project,
      legacy = false
    )
  }

  // Returns first ID found
  def findFirst(filter: String,
                orderBy: Option[String] = None,
                cursor: Option[String] = None,
                take: Option[String] = None,
                skip: Option[String] = None): Double = {

    val result = server.query(
      s"""{
        |  findFirstTestModel(
        |    where: $filter
        |    ${formatArg("orderBy", orderBy)}
        |    ${formatArg("cursor", cursor)}
        |    ${formatArg("take", take)}
        |    ${formatArg("skip", skip)}
        |  ) {
        |    id
        |  }
        |}""".stripMargin,
      project,
      legacy = false
    )

    result.pathAsInt("data.findFirstTestModel.id")
  }

  def formatArg(name: String, arg: Option[String]): String = {
    arg.map(a => s"$name: $a").getOrElse("")
  }
}
