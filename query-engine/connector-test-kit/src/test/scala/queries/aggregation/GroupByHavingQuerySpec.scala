package queries.aggregation

import org.scalatest.{FlatSpec, Matchers}
import util._

// Testing assumptions
// - Grouping on fields itself works (as tested in the GroupBySpec).
// - The above means we also don't need to test combinations except for what is required by the rules to make it work.
// - It also means we don't need to test the selection of aggregates extensively beyond the need to sanity check the group filter.
// - We don't need to check every single filter operation, as it's ultimately the same code path just with different
//   operators applied. For a good confidence, we choose `equals`, `in`, `not equals`, `endsWith` (where applicable).
class GroupByHavingQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model Model {
      |  id    String  @id @default(cuid())
      |  float Float?   @map("db_float")
      |  int   Int?     @map("db_int")
      |  dec   Decimal? @map("db_dec")
      |  s     String?  @map("db_s")
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def create(float: Option[Double], int: Option[Int], dec: Option[String], s: String, id: Option[String] = None) = {
    val idString = id match {
      case Some(i) => s"""id: "$i","""
      case None    => ""
    }

    server.query(
      s"""mutation {
         |  createModel(data: {
         |    $idString
         |    float: ${float.getOrElse("null")}
         |    int: ${int.getOrElse("null")}
         |    dec: ${dec.map(d => s""""$d"""").getOrElse("null")}
         |    s: "$s"
         |  }) {
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
    // Float, int, dec, s, id
    create(Some(10.1), Some(5), Some("1.1"), "group1", Some("1"))
    create(Some(5.5), Some(0), Some("6.7"), "group1", Some("2"))
    create(Some(10), Some(5), Some("11"), "group2", Some("3"))
    create(Some(10), Some(5), Some("11"), "group3", Some("4"))

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

  "Using a groupBy with a `having` scalar filters on list fields" should "work" taggedAs (IgnoreMySql, IgnoreSQLite, IgnoreMsSql) in {
    val project = SchemaDsl.fromStringV11() {
      """model Model {
        |  id   Int   @id @default(autoincrement())
        |  ints Int[]
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """mutation {
        |  createOneModel(data: { ints: [1, 2, 3] }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    // Group 1 and 2 returned
    var result = server.query(
      s"""{
         |  groupByModel(by: [ints], having: {
         |    ints: { equals: [1, 2, 3] }
         |  }) {
         |    ints
         |    count {
         |      ints
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"ints":[1,2,3],"count":{"ints":1}}]}}""")
  }

  // *************
  // *** Count ***
  // *************

  "Using a groupBy with a `having` count scalar filters" should "work" in {
    // Float, int, dec, s, id
    create(None, Some(1), None, "group1", Some("1"))
    create(None, Some(2), None, "group1", Some("2"))
    create(None, Some(3), None, "group2", Some("3"))
    create(None, None, None, "group2", Some("4"))
    create(None, None, None, "group3", Some("5"))
    create(None, None, None, "group3", Some("6"))

    // Group 1 has 2, 2 has 1, 3 has 0
    var result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    int: {
         |      count: {
         |        equals: 2
         |      }
         |    }
         |  }) {
         |    s
         |    count {
         |      int
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group1","count":{"int":2}}]}}""")

    // Group 2 and 3 returned
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    int: {
         |      count: {
         |        not: { equals: 2 }
         |      }
         |    }
         |  }) {
         |    s
         |    count {
         |      int
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group2","count":{"int":1}},{"s":"group3","count":{"int":0}}]}}""")

    // Group 1 and 3 returned
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    int: {
         |      count: {
         |        in: [0, 2]
         |      }
         |    }
         |  }) {
         |    s
         |    count {
         |      int
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group1","count":{"int":2}},{"s":"group3","count":{"int":0}}]}}""")
  }

  // ***************
  // *** Average ***
  // ***************

  "Using a groupBy with a `having` average scalar filters" should "work" in {
    // Float, int, dec, s, id
    create(None, Some(10), Some("10"), "group1", Some("1"))
    create(None, Some(6), Some("6"), "group1", Some("2"))
    create(None, Some(3), Some("5"), "group2", Some("3"))
    create(None, None, None, "group2", Some("4"))
    create(None, None, None, "group3", Some("5"))
    create(None, None, None, "group3", Some("6"))

    // Group 1 has 8, 2 has 5, 3 has 0
    var result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    dec: {
         |      avg: {
         |        equals: "8.0"
         |      }
         |    }
         |  }) {
         |    s
         |    avg {
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group1","avg":{"dec":"8"}}]}}""")

    // Group 2 and 3 returned (3 is null)
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    OR: [
         |      { dec: { avg: { not: { equals: "8.0" }}}},
         |      { dec: { avg: { equals: null }}}
         |    ]}
         |  ) {
         |      s
         |      avg {
         |        dec
         |      }
         |    }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group2","avg":{"dec":"5"}},{"s":"group3","avg":{"dec":"0"}}]}}""")

    // Group 1 and 2 returned
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    dec: {
         |      avg: {
         |        in: ["8", "5"]
         |      }
         |    }
         |  }) {
         |    s
         |    avg {
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group1","avg":{"dec":"8"}},{"s":"group2","avg":{"dec":"5"}}]}}""")
  }

  // ***********
  // *** Sum ***
  // ***********

  "Using a groupBy with a `having` sum scalar filters" should "work" in {
    // Float, int, dec, s, id
    create(Some(10), Some(10), Some("10"), "group1", Some("1"))
    create(Some(6), Some(6), Some("6"), "group1", Some("2"))
    create(Some(5), Some(5), Some("5"), "group2", Some("3"))
    create(None, None, None, "group2", Some("4"))
    create(None, None, None, "group3", Some("5"))
    create(None, None, None, "group3", Some("6"))

    // Group 1 has 16, 2 has 6, 3 has 0
    var result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { sum: { equals: 16 }}
         |    int: { sum: { equals: 16 }}
         |    dec: { sum: { equals: "16" }}
         |  }) {
         |    s
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group1","sum":{"float":16,"int":16,"dec":"16"}}]}}""")

    // Group 2 (3 is null)
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { sum: { not: { equals: 16 }}}
         |    int: { sum: { not: { equals: 16 }}}
         |    dec: { sum: { not: { equals: "16" }}}
         |  }) {
         |    s
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[{"s":"group2","sum":{"float":5,"int":5,"dec":"5"}}]}}""")

    // Group 1 and 2 returned
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { sum: { in: [16, 5] }}
         |    int: { sum: { in: [16, 5] }}
         |    dec: { sum: { in: ["16", "5"] }}
         |  }) {
         |    s
         |    sum {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","sum":{"float":16,"int":16,"dec":"16"}},{"s":"group2","sum":{"float":5,"int":5,"dec":"5"}}]}}""")
  }

  // ***********
  // *** Min ***
  // ***********

  "Using a groupBy with a `having` min scalar filters" should "work" in {
    // Float, int, dec, s, id
    create(Some(10), Some(10), Some("10"), "group1", Some("1"))
    create(Some(0), Some(0), Some("0"), "group1", Some("2"))
    create(Some(0), Some(0), Some("0"), "group2", Some("3"))
    create(None, None, None, "group2", Some("4"))
    create(None, None, None, "group3", Some("5"))
    create(None, None, None, "group3", Some("6"))

    // Group 1 and 2 returned
    var result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { min: { equals: 0 }}
         |    int: { min: { equals: 0 }}
         |    dec: { min: { equals: "0" }}
         |  }) {
         |    s
         |    min {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","min":{"float":0,"int":0,"dec":"0"}},{"s":"group2","min":{"float":0,"int":0,"dec":"0"}}]}}""")

    // Empty
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { min: { not: { equals: 0 }}}
         |    int: { min: { not: { equals: 0 }}}
         |    dec: { min: { not: { equals: "0" }}}
         |  }) {
         |    s
         |    min {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[]}}""")

    // Group 1 and 2 returned
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { min: { in: [0] }}
         |    int: { min: { in: [0] }}
         |    dec: { min: { in: ["0"] }}
         |  }) {
         |    s
         |    min {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","min":{"float":0,"int":0,"dec":"0"}},{"s":"group2","min":{"float":0,"int":0,"dec":"0"}}]}}""")
  }

  // ***********
  // *** Max ***
  // ***********

  "Using a groupBy with a `having` max scalar filters" should "work" in {
    // Float, int, dec, s, id
    create(Some(10), Some(10), Some("10"), "group1", Some("1"))
    create(Some(0), Some(0), Some("0"), "group1", Some("2"))
    create(Some(10), Some(10), Some("10"), "group2", Some("3"))
    create(None, None, None, "group2", Some("4"))
    create(None, None, None, "group3", Some("5"))
    create(None, None, None, "group3", Some("6"))

    // Group 1 returned
    var result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { max: { equals: 10 }}
         |    int: { max: { equals: 10 }}
         |    dec: { max: { equals: "10" }}
         |  }) {
         |    s
         |    max {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","max":{"float":10,"int":10,"dec":"10"}},{"s":"group2","max":{"float":10,"int":10,"dec":"10"}}]}}""")

    // Empty
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { max: { not: { equals: 10 }}}
         |    int: { max: { not: { equals: 10 }}}
         |    dec: { max: { not: { equals: "10" }}}
         |  }) {
         |    s
         |    max {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be("""{"data":{"groupByModel":[]}}""")

    // Group 1 and 2 returned
    result = server.query(
      s"""{
         |  groupByModel(by: [s], orderBy: { s: asc }, having: {
         |    float: { max: { in: [10] }}
         |    int: { max: { in: [10] }}
         |    dec: { max: { in: ["10"] }}
         |  }) {
         |    s
         |    max {
         |      float
         |      int
         |      dec
         |    }
         |  }
         |}""".stripMargin,
      project
    )
    result.toString should be(
      """{"data":{"groupByModel":[{"s":"group1","max":{"float":10,"int":10,"dec":"10"}},{"s":"group2","max":{"float":10,"int":10,"dec":"10"}}]}}""")
  }

  // *******************
  // *** Error cases ***
  // *******************

  "Using a groupBy with a `having` scalar filter that mismatches the selection" should "throw an error" in {
    server.queryThatMustFail(
      s"""{
           |  groupByModel(by: [s], having: { int: { gt: 3 } }) {
           |    sum {
           |      int
           |    }
           |  }
           |}""".stripMargin,
      project,
      errorCode = 2019,
      errorContains =
        "Every field used in `having` filters must either be an aggregation filter or be included in the selection of the query. Missing fields: int"
    )
  }
}
