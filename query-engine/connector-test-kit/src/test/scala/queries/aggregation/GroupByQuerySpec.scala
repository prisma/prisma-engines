package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

// RS: Ported
class GroupByQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Model {
      |  id       String  @id @default(cuid())
      |  float    Float   @map("db_float")
      |  int      Int     @map("db_int")
      |  dec      Decimal @map("db_dec")
      |  s        String  @map("db_s")
      |  otherId  Int?
      |  other    Other?  @relation(fields: otherId, references: id)
      |}
      |
      |model Other {
      |  id       Int     @id
      |  field    String
      |  model    Model[]
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
         |    _count { id }
         |    float
         |    _sum { int }
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
         |    _count { s }
         |    _sum { float }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","_count":{"s":2},"_sum":{"float":15.6}},{"s":"group2","_count":{"s":1},"_sum":{"float":10}},{"s":"group3","_count":{"s":1},"_sum":{"float":10}}]}}""")
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
         |    _count { s }
         |    _sum { float }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","_count":{"s":1},"_sum":{"float":10}},{"s":"group2","_count":{"s":1},"_sum":{"float":10}},{"s":"group1","_count":{"s":2},"_sum":{"float":15.6}}]}}""")
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
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","_count":{"s":2},"_sum":{"float":25},"_min":{"int":5}},{"s":"group2","_count":{"s":1},"_sum":{"float":10},"_min":{"int":5}},{"s":"group1","_count":{"s":1},"_sum":{"float":5.5},"_min":{"int":0}},{"s":"group1","_count":{"s":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}""")
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
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is the first one with 2, then group2 with one, then group1 with 1.
    // group2 is returned, because group3 is skipped.
    result.toString should be("""{"data":{"groupByModel":[{"s":"group2","_count":{"s":1},"_sum":{"float":10},"_min":{"int":5}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, take: -1, skip: 2) {
         |    s
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is the first one with 2, then group2 with one, then group1 with 1.
    // group3 is returned, because group1 and 2 is skipped (reverse order due to negative take).
    result.toString should be("""{"data":{"groupByModel":[{"s":"group3","_count":{"s":2},"_sum":{"float":25},"_min":{"int":5}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, take: 2, skip: 1) {
         |    s
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // group3 is the first one with 2, then group2 with one, then group1 with 1.
    // group2 & 1 are returned, because group3 is skipped.
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group2","_count":{"s":1},"_sum":{"float":10},"_min":{"int":5}},{"s":"group1","_count":{"s":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}""")
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
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Group3 has only id 4, id 5 is filtered.
    // Group2 has id 3.
    // Group1 id 1, id 2 is filtered.
    // => All groups have count 1
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","_count":{"s":1},"_sum":{"float":10},"_min":{"int":5}},{"s":"group2","_count":{"s":1},"_sum":{"float":10},"_min":{"int":5}},{"s":"group1","_count":{"s":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}""")
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
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Group3 has 2
    // Group2 has 0
    // Group1 has 1
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group3","_count":{"s":2},"_sum":{"float":25},"_min":{"int":5}},{"s":"group1","_count":{"s":1},"_sum":{"float":10.1},"_min":{"int":5}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [s, int], orderBy: { s: desc }, where: {
         |    other: { is: { field: { equals: "b" }}}
         |  }) {
         |    s
         |    _count { s }
         |    _sum { float }
         |    _min { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Group3 has 2 matches
    // Group2 has 0 matches
    // Group1 has 0 matches
    result.toString should be("""{"data":{"groupByModel":[{"s":"group3","_count":{"s":2},"_sum":{"float":25},"_min":{"int":5}}]}}""")
  }

  "Using a group-by with ordering COUNT aggregation" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 2, "11", "group1", Some("2"))
    create(1.1, 3, "3", "group2", Some("3"))
    create(4.0, 3, "4", "group3", Some("4"))

    var result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _count: { float: asc } }) {
         |    float
         |    _count {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":4,"_count":{"float":1}},{"float":1.1,"_count":{"float":3}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _count: { float: desc } }) {
         |    float
         |    _count {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":1.1,"_count":{"float":3}},{"float":4,"_count":{"float":1}}]}}""")
  }

  "Using a group-by with ordering SUM aggregation" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 2, "11", "group1", Some("2"))
    create(1.1, 3, "3", "group2", Some("3"))
    create(4.0, 3, "4", "group3", Some("4"))

    var result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _sum: { float: asc } }) {
         |    float
         |    _sum {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":1.1,"_sum":{"float":3.3}},{"float":4,"_sum":{"float":4}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _sum: { float: desc } }) {
         |    float
         |    _sum {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":4,"_sum":{"float":4}},{"float":1.1,"_sum":{"float":3.3}}]}}""")
  }

  "Using a group-by with ordering AVG aggregation" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 2, "11", "group1", Some("2"))
    create(1.1, 3, "3", "group2", Some("3"))
    create(4.0, 3, "4", "group3", Some("4"))

    var result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _avg: { float: asc } }) {
         |    float
         |    _avg {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":1.1,"_avg":{"float":1.1}},{"float":4,"_avg":{"float":4}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _avg: { float: desc } }) {
         |    float
         |    _avg {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":4,"_avg":{"float":4}},{"float":1.1,"_avg":{"float":1.1}}]}}""")
  }

  "Using a group-by with ordering MIN aggregation" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 2, "11", "group1", Some("2"))
    create(1.1, 3, "3", "group2", Some("3"))
    create(4.0, 3, "4", "group3", Some("4"))

    var result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _min: { float: asc } }) {
         |    float
         |    _min {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":1.1,"_min":{"float":1.1}},{"float":4,"_min":{"float":4}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _min: { float: desc } }) {
         |    float
         |    _min {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":4,"_min":{"float":4}},{"float":1.1,"_min":{"float":1.1}}]}}""")
  }

  "Using a group-by with ordering MAX aggregation" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 2, "11", "group1", Some("2"))
    create(1.1, 3, "3", "group2", Some("3"))
    create(4.0, 3, "4", "group3", Some("4"))

    var result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _max: { float: asc } }) {
         |    float
         |    _max {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":1.1,"_max":{"float":1.1}},{"float":4,"_max":{"float":4}}]}}""")

    result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _max: { float: desc } }) {
         |    float
         |    _max {
         |      float
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"float":4,"_max":{"float":4}},{"float":1.1,"_max":{"float":1.1}}]}}""")
  }

  "Using a group-by with multiple ordering aggregation of different fields" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 1, "11", "group1", Some("2"))
    create(1.1, 1, "3", "group2", Some("3"))
    create(3.0, 3, "4", "group3", Some("5"))
    create(4.0, 4, "4", "group3", Some("4"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [float, int], orderBy: [{ _count: { float: desc } }, { _sum: { int: asc } }]) {
         |    float
         |    _count { float }
         |    _sum { int }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"float":1.1,"_count":{"float":3},"_sum":{"int":3}},{"float":3,"_count":{"float":1},"_sum":{"int":3}},{"float":4,"_count":{"float":1},"_sum":{"int":4}}]}}""")
  }

  "Using a group-by with multiple ordering aggregation and having" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 1, "11", "group1", Some("2"))
    create(1.1, 1, "3", "group2", Some("3"))
    create(3.0, 3, "4", "group3", Some("5"))
    create(4.0, 4, "4", "group3", Some("4"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [float, int], orderBy: [{ _count: { float: desc } }, { _sum: { int: asc } }], having: { float: { lt: 4 } }) {
         |    float
         |    _count { float }
         |    _sum { int }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be(
      """{"data":{"groupByModel":[{"float":1.1,"_count":{"float":3},"_sum":{"int":3}},{"float":3,"_count":{"float":1},"_sum":{"int":3}}]}}""")
  }

  "Using a group-by with order by aggregation without selecting the ordered field" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 1, "11", "group1", Some("1"))
    create(1.1, 1, "11", "group1", Some("2"))
    create(1.1, 1, "3", "group2", Some("3"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [float], orderBy: { _count: { float: desc } }) {
         |    _sum { int }
         |  }
         |}""".stripMargin,
      project
    )

    result.toString should be("""{"data":{"groupByModel":[{"_sum":{"int":3}}]}}""")
  }

  "Using a group-by with order by aggregation without selecting the ordered field or grouping by it" should "work" in {
    // Float, int, dec, s, id
    create(1.1, 4, "11", "group1", Some("1"))
    create(1.1, 1, "11", "group1", Some("2"))
    create(1.1, 1, "3", "group2", Some("3"))

    val result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { _sum: { int: asc } }) {
         |    s
         |    _sum { int }
         |  }
         |}""".stripMargin,
      project
    )

    // Ordered by number of records ASC (basically, since int is not null):
    // Group 2 with 1 (sum is 4)
    // Group 1 with 2 (sum is 2)
    result.toString should be("""{"data":{"groupByModel":[{"s":"group2","_sum":{"int":1}},{"s":"group1","_sum":{"int":5}}]}}""")
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
         |    _count { s }
         |    _sum { float }
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
         |    _count { int }
         |    _sum { float }
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
         |    _sum
         |  }
         |}""".stripMargin,
      project,
      errorCode = 2009,
      errorContains = "Expected a minimum of 1 fields to be present, got 0."
    )
  }
}
