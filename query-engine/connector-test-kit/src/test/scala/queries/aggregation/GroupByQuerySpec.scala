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
      |  other Other?
      |}
      |
      |model Other {
      |  id    Int    @id
      |  field String
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def create(float: Double, int: Int, dec: String, s: String, id: Option[String] = None, other: Option[(Int, String)] = None) = {
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

  "Using a groupBy without any records in the database" should "return no groups" in {
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
    // Float, int, dec, s, id
    create(10.1, 5, "1.1", "group1", Some("1"))
    create(5.5, 0, "6.7", "group1", Some("2"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "11", "group3", Some("4"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }) {
         |    s
         |    count { s }
         |    sum { float }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","count":{"s":2},"sum":{"float":15.6}},{"s":"group2","count":{"s":1},"sum":{"float":10}},{"s":"group3","count":{"s":1},"sum":{"float":10}}]}}""")
  }

  "Using a groupBy with reverse ordering" should "return the correct groups" in {
    // Float, int, dec, s, id
    create(10.1, 5, "1.1", "group1", Some("1"))
    create(5.5, 0, "6.7", "group1", Some("2"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "11", "group3", Some("4"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: desc }) {
         |    s
         |    count { s }
         |    sum { float }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","count":{"s":1},"sum":{"float":10}},{"s":"group2","count":{"s":1},"sum":{"float":10}},{"s":"group1","count":{"s":2},"sum":{"float":15.6}}]}}""")
  }

  "Using a groupBy with multiple orderings" should "return the correct groups" in {
    // Float, int, dec, s, id
    create(10.1, 5, "1.1", "group1", Some("1"))
    create(5.5, 0, "6.7", "group1", Some("2"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "11", "group3", Some("4"))
    create(15, 5, "11", "group3", Some("5"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: [{ s: desc }, { int: asc }]) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","count":{"s":2},"sum":{"float":25},"min":{"int":5}},{"s":"group2","count":{"s":1},"sum":{"float":10},"min":{"int":5}},{"s":"group1","count":{"s":1},"sum":{"float":5.5},"min":{"int":0}},{"s":"group1","count":{"s":1},"sum":{"float":10.1},"min":{"int":5}}]}}""")
  }

  "Using a groupBy with take and skip" should "return the correct groups" in {
    // Float, int, dec, s, id
    create(10.1, 5, "1.1", "group1", Some("1"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "11", "group3", Some("4"))
    create(15, 5, "11", "group3", Some("5"))

    var result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, take: 1, skip: 1) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is the first one with 2, then group2 with one, then group1 with 1.
    // group2 is returned, because group3 is skipped.
    result.toString should be("""{"data":{"groupByModel":[{"s":"group2","count":{"s":1},"sum":{"float":10},"min":{"int":5}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, take: -1, skip: 2) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is the first one with 2, then group2 with one, then group1 with 1.
    // group3 is returned, because group1 and 2 is skipped (reverse order due to negative take).
    result.toString should be("""{"data":{"groupByModel":[{"s":"group3","count":{"s":2},"sum":{"float":25},"min":{"int":5}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, take: 2, skip: 1) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is the first one with 2, then group2 with one, then group1 with 1.
    // group2 & 1 are returned, because group3 is skipped.
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group2","count":{"s":1},"sum":{"float":10},"min":{"int":5}},{"s":"group1","count":{"s":1},"sum":{"float":10.1},"min":{"int":5}}]}}""")
  }

  "Using a groupBy with scalar filters" should "return the correct groups" in {
    // What this test checks: Scalar filters apply before the grouping is done,
    // changing the aggregations of the groups, not the groups directly.
    // Float, int, dec, s, id
    create(10.1, 5, "1.1", "group1", Some("1"))
    create(5.5, 0, "6.7", "group1", Some("2"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "13", "group3", Some("4"))
    create(15, 5, "10", "group3", Some("5"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, where: {
         |    int: 5,
         |    float: { lt: 15 }
         |  }) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Group3 has only id 4, id 5 is filtered.
    // Group2 has id 3.
    // Group1 id 1, id 2 is filtered.
    // => All groups have count 1
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","count":{"s":1},"sum":{"float":10},"min":{"int":5}},{"s":"group2","count":{"s":1},"sum":{"float":10},"min":{"int":5}},{"s":"group1","count":{"s":1},"sum":{"float":10.1},"min":{"int":5}}]}}""")
  }

  "Using a groupBy with relation filters" should "return the correct groups" in {
    // What this test checks: Scalar filters apply before the grouping is done,
    // changing the aggregations of the groups, not the groups directly.
    // Float, int, dec, s, id
    create(10.1, 5, "1.1", "group1", Some("1"), other = Some((1, "a")))
    create(5.5, 0, "6.7", "group1", Some("2"))
    create(10, 5, "11", "group2", Some("3"))
    create(10, 5, "13", "group3", Some("4"), other = Some((2, "b")))
    create(15, 5, "10", "group3", Some("5"), other = Some((3, "b")))

    var result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, where: {
         |    other: { isNot: null }
         |  }) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Group3 has 2
    // Group2 has 0
    // Group1 has 1
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","count":{"s":2},"sum":{"float":25},"min":{"int":5}},{"s":"group1","count":{"s":1},"sum":{"float":10.1},"min":{"int":5}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, where: {
         |    other: { is: { field: { equals: "b" }}}
         |  }) {
         |    s
         |    count { s }
         |    sum { float }
         |    min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Group3 has 2 matches
    // Group2 has 0 matches
    // Group1 has 0 matches
    result.toString should be("""{"data":{"groupByModel":[{"s":"group3","count":{"s":2},"sum":{"float":25},"min":{"int":5}}]}}""")
  }

  /////// Error Cases

  "Using a groupBy without any by selection" should "error" in {
    server.queryThatMustFail(
      s"""{
         |  groupByModel(by: []) {
         |    s
         |  }
         |}""".stripMargin,
      project,
      errorCode = 2019,
      errorContains = "At least one selection is required for the `by` argument."
    )
  }

  "Using a groupBy with mismatching by-arguments and query selections" should "return an error detailing the missing fields" in {
    server.queryThatMustFail(
      s"""{
         |  groupByModel(by: [int]) {
         |    s
         |    count { s }
         |    sum { float }
         |  }
         |}""".stripMargin,
      project,
      errorCode = 2019,
      errorContains = "Every selected scalar field that is not part of an aggregation must be included in the by-arguments of the query. Missing fields: s"
    )
  }

  "Using a groupBy with mismatching by-arguments and orderBy" should "return an error detailing the missing fields" in {
    server.queryThatMustFail(
      s"""{
         |  groupByModel(by: [int], orderBy: { s: desc }) {
         |    count { int }
         |    sum { float }
         |  }
         |}""".stripMargin,
      project,
      errorCode = 2019,
      errorContains = "Every field used for orderBy must be included in the by-arguments of the query. Missing fields: s"
    )
  }

  "Using a groupBy with an empty aggregation selection" should "throw an appropriate error" in {
    server.queryThatMustFail(
      s"""{
         |  groupByModel(by: [s]) {
         |    sum
         |  }
         |}""".stripMargin,
      project,
      errorCode = 2009,
      errorContains = "Expected a minimum of 1 fields to be present, got 0."
    )
  }
}
