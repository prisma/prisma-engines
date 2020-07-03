package queries.distinct

import org.scalatest.{FlatSpec, Matchers}
import util._

class DistinctQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model TestModel {
      |  id     String @id @default(cuid())
      |  fieldA String
      |  fieldB Int
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createRecord(fieldA: String, fieldB: Int) = {
    server.query(
      s"""mutation {
         |  createOneTestModel(data: { fieldA: "$fieldA", fieldB: $fieldB }) {
         |    id
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )
  }

  "Select distinct with no records in the database" should "return nothing" in {
    val result = server.query(
      s"""{
         |  findManyTestModel(distinct: true) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  "Select distinct with a few duplicates in the database" should "return only distinct records" in {
    createRecord("1", 1)
    createRecord("2", 2)
    createRecord("1", 1)

    val result = server.query(
      s"""{
         |  findManyTestModel(distinct: true) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }
}
