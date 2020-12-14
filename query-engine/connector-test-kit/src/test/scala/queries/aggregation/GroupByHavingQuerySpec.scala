package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

class GroupByHavingQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  case class FieldDef(name: String, mappingType: MappingType, numeric: Boolean)

  trait MappingType {
    val inner: String
  }

  case class StringMappingType(inner: String) extends MappingType
  case class NumberMappingType(inner: String) extends MappingType

  // 1) Define fields to include in the test.
  // 2) Generate model and project based on model.
  // 3) Generate test cases. For every field in the query:
  //    - Generate x random groups with size 1..y ON THAT FIELD
  //    - Calculate aggregations on these groups for checks.
  //    - For each possible aggregation type for that field:
  //      - For each possible scalar filter for that aggregation filter:
  //        - Generate and execute query with having filter.
  //        - Check the resulting groups against pre-calculated groups, applying the filter in-memory.
  //
  // If something fails, print all info for reproduction.
  def generateCases(): Unit = {
    val modelFields = Seq(
      FieldDef("float", NumberMappingType("Float"), numeric = true),
      FieldDef("int", NumberMappingType("Int"), numeric = true),
      FieldDef("dec", StringMappingType("Decimal"), numeric = true),
      FieldDef("bigInt", StringMappingType("BigInt"), numeric = true),
      FieldDef("str", StringMappingType("String"), numeric = true)

    val datamodelString = generateDatamodel(modelFields)
    val project         = SchemaDsl.fromStringV11()(datamodelString)

    
  }

  def generateDatamodel(fields: Seq[FieldDef]): String = {
    val stringified = fields.map(field => {
      s"""${field.name} ${field.mappingType.inner} @map("db_${field.name}")"""
    })

    s"""model Model {
        |  id    String  @id @default(cuid())
        |  ${stringified.mkString("\n")}
        |}
      """.stripMargin
  }

  def create(project: Project, float: Double, int: Int, dec: String, s: String, id: Option[String] = None, other: Option[(Int, String)] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    val stringifiedOther = other match {
      case Some(other) => s""", other: { create: { id: ${other._1}, field: "${other._2}" } }"""
      case None        => ""
    }

    server.query(
      s"""mutation {
         |  createModel(data: { $idString float: $float, int: $int, dec: $dec, s: "$s" $stringifiedOther }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )
  }

  // This is just basic confirmation that scalar filters are applied correctly.
  // The assumption is that we don't need to test all normal scalar filters as they share the exact same code path
  // and are extracted and applied exactly as the already tested ones. This also extends to AND/OR/NOT combinators.
  // Consequently, subsequent tests in this file will deal exclusively with the newly added aggregation filters.
  "Using a groupBy with a basic `having` scalar filter" should "work" in {
    val project = SchemaDsl.fromStringV11() {
      """model Model {
        |  id    String  @id @default(cuid())
        |  float Float   @map("db_float")
        |  int   Int     @map("db_int")
        |  dec   Decimal @map("db_dec")
        |  s     String  @map("db_s")
        |  other Other?
        |}
        |
        |model Other {
        |  id    Int    @id
        |  field String
        |}
      """.stripMargin
    }
    database.setup(project)

    // Float, int, dec, s, id
    create(project, 10.1, 5, "1.1", "group1", Some("1"))
    create(project, 5.5, 0, "6.7", "group1", Some("2"))
    create(project, 10, 5, "11", "group2", Some("3"))
    create(project, 10, 5, "11", "group3", Some("4"))

    // Group [s, int] produces:
    // group1, 5
    // group1, 0
    // group2, 5
    // group3, 5
    val result = server.query(
      s"""{
         |  groupByModel(by: [s, int], having: {
         |    s: { in: ["group1", "group2"] }
         |    int: 5
         |  }) {
         |    s
         |    int
         |    count { _all }
         |    sum { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is filtered completely, group1 (int 0) is filtered as well.
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","int":5,"count":{"_all":1},"sum":{"int":5}},{"s":"group2","int":5,"count":{"_all":1},"sum":{"int":5}}]}}""")
  }
}
