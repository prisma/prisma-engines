package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

import scala.collection.mutable.{ArrayBuffer, HashMap}

class GroupByHavingQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  case class FieldDef(name: String, mappingType: MappingType, numeric: Boolean)

  trait MappingType {
    val prismaType: String
  }

  case class StringMappingType(prismaType: String) extends MappingType
  case class NumberMappingType(prismaType: String) extends MappingType

  case class Group(id: Int, rows: ArrayBuffer[ArrayBuffer[Double]], aggregations: HashMap[String, HashMap[String, Double]])

  trait AggregationOperation {
    val op: String
  }

  case class Count(op: String = "count") extends AggregationOperation
  case class Average(op: String = "avg") extends AggregationOperation
  case class Sum(op: String = "sum")     extends AggregationOperation
  case class Min(op: String = "min")     extends AggregationOperation
  case class Max(op: String = "max")     extends AggregationOperation

  val possibleAggregations: Seq[AggregationOperation] = Seq(Count(), Average(), Sum(), Min(), Max())

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
  def generateAndExecuteTestMatrix(): Unit = {
    val modelFields = Seq(
      FieldDef("float", NumberMappingType("Float"), numeric = true),
      FieldDef("int", NumberMappingType("Int"), numeric = true),
      FieldDef("dec", StringMappingType("Decimal"), numeric = true),
      FieldDef("bigInt", StringMappingType("BigInt"), numeric = true),
      FieldDef("str", StringMappingType("String"), numeric = false)
    )

    val datamodelString = generateDatamodel(modelFields)
    val project         = SchemaDsl.fromStringV11()(datamodelString)

    database.setup(project)
    database.truncateProjectTables(project)

    for (field <- modelFields) yield {
      generateGroups(project, field, modelFields)
      executeFieldTests(field)
    }
  }

  def generateDatamodel(fields: Seq[FieldDef]): String = {
    // Note: All fields are optional to enable null field tests.
    val stringified = fields.map(field => {
      s"""${field.name} ${field.mappingType.prismaType}? @map("db_${field.name}")"""
    })

    s"""model Model {
        |  id Int @id @default(autoincrement())
        |  ${stringified.mkString("\n")}
        |}
      """.stripMargin
  }

  // The `on` field passed acts as the group ID storage.
  def generateGroups(project: Project, on: FieldDef, fields: Seq[FieldDef]): Unit = {
    val random = scala.util.Random

    // In-memory aggregations for the generated groups.
    // Group ID -> For each field (column) a map of aggregations ("min", "max", "avg", "count", "sum").
    // If the aggregation map for a column is missing an aggregation, it's null (None).
    val aggregationMap = new HashMap[Int, HashMap[String, HashMap[String, Option[Double]]]]()

    // 5 - 10 groups
    val numGroups = Math.min(5, random.nextInt(10))
    println(s"Generating $numGroups groups.")

    for (groupId <- 1 until numGroups) yield {
      val groupAggregationMap = new HashMap[String, HashMap[String, Option[Double]]]()
      for (field <- fields) yield {
        groupAggregationMap.put(field.name, new HashMap())
      }

      aggregationMap.put(groupId, groupAggregationMap)

      // Generate 1 - 10 rows for each group.
      val numRows = Math.max(1, random.nextInt(10))
      println(s"[$groupId] Generating $numRows rows.")

      // Rows for this group. Columns orderd by `fields`.
      val rows = new ArrayBuffer[ArrayBuffer[Option[Double]]]()

      for (rowNum <- 0 until numRows) yield {
        val row = new ArrayBuffer[Option[Double]]()

        // For each field, generate a value or null.
        // Distribution: If value is equal or above 100 in a [0, 125) interval, it's null (20%).
        for (field <- fields) yield {
          if (field != on) {
            val fieldValue = random.nextInt(125)

            if (fieldValue < 100) {
              row.append(Some(fieldValue))
            } else {
              println(">>>>>>>>>>>>>>>>>>>>> INSERTING NULL")
              row.append(None)
            }
          } else {
            row.append(Some(groupId))
          }
        }

        rows.append(row)
      }

      // Compute aggregations.
      for ((field, i) <- fields.zipWithIndex) yield {
        val values = columnValues(rows, i)

        for (aggregation <- possibleAggregations) yield {
          if (values.isEmpty) {

            aggregation match {
              case Count(op) =>
                val map = groupAggregationMap(field.name)
                map.put(op, Some(0))

              case agg =>
                val map = groupAggregationMap(field.name)
                map.put(agg.op, None)
            }
          } else {
            aggregation match {
              case Count(op) =>
                val map = groupAggregationMap(field.name)
                map.put(op, Some(values.length))

              case Average(op) =>
                val map = groupAggregationMap(field.name)
                map.put(op, Some(values.sum / values.length))

              case Sum(op) =>
                val map = groupAggregationMap(field.name)
                map.put(op, Some(values.sum))

              case Min(op) =>
                val map = groupAggregationMap(field.name)
                map.put(op, Some(values.min))

              case Max(op) =>
                val map = groupAggregationMap(field.name)
                map.put(op, Some(values.max))
            }
          }
        }
      }

      // Create rows in DB
      createRows(project, rows, fields)
    }
  }

  // Returns all non-null (None) values for a column index.
  def columnValues(rows: ArrayBuffer[ArrayBuffer[Option[Double]]], index: Int): Seq[Double] = {
    val values = for (row <- rows) yield {
      row(index)
    }

    values.filter(v => v.isDefined).map(_.get)
  }

  def createRows(project: Project, rows: ArrayBuffer[ArrayBuffer[Option[Double]]], fields: Seq[FieldDef]) = {
    for (row <- rows) yield {
      val zipped: Seq[(FieldDef, Option[Double])] = fields.zip(row)

      val values = zipped.map {
        case (field, value) =>
          field.mappingType match {
            case StringMappingType(_) if value.isDefined => s"""${field.name}: "${value.get}""""
            case _ if value.isEmpty                      => s"""${field.name}: null"""
            case _                                       => s"${field.name}: ${value.get}"
          }
      }

      server.query(
        s"""mutation {
           |  createModel(data: { ${values.mkString(", ")} }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
    }
  }

  def executeFieldTests(field: FieldDef): Unit = {}

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

  "Valid single-aggregation having filters" should "work as expected" in {
    generateAndExecuteTestMatrix()
  }
}
