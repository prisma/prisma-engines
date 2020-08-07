package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.EnumCapability
import util._

class PortedFiltersSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(EnumCapability)

  // Always running the filter query twice - once with a one relation condition that is always true for all nodes - ensures that
  // Mongo executes the query once as a find query and once using the aggregation framework

  val project: Project = ProjectDsl.fromString { """
    |model ScalarModel {
    |  id          String  @id @default(cuid())
    |  idTest      String?
    |  optString   String?
    |  optInt      Int?
    |  optFloat    Float?
    |  optBoolean  Boolean?
    |  optDateTime DateTime?
    |  optEnum     Enum?
    |  b_id        String?
    |
    |  b B? @relation(fields: [b_id], references: [id])
    |}
    |
    |model B {
    | id  String @id @default(cuid())
    | int Int?   @unique
    |}
    |
    |enum Enum{
    | A
    | B
    |}
    |""" }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.truncateProjectTables(project)
    server.query(s"""mutation{createB(data:{int: 1}){id}}""".stripMargin, project)
  }

  def createTest(id: String, optString: String, optInt: Int, optFloat: Float, optBoolean: Boolean, optEnum: String, optDateTime: String): Unit = {
    val string = if (optString == null) "null" else s""""$optString""""

    server.query(
      s"""mutation{createScalarModel(data:{
         |idTest:"$id",
         |optString: $string,
         |optInt: $optInt,
         |optFloat: $optFloat,
         |optBoolean: $optBoolean,
         |optEnum: $optEnum,
         |optDateTime: "$optDateTime"
         |b:{connect:{int: 1}}}){id}}""".stripMargin,
      project
    )
  }

  //region Recursion
  "A filter query" should "support the AND filter in one recursion level" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id4", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id5", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res =
      server.query("""{scalarModels(where: {optString: { starts_with: "foo" }, AND: [{optBoolean: false, idTest: { ends_with: "5" }}]}){idTest}}""",
                   project = project)

    val res2 =
      server.query(
        """{scalarModels(where: {b: {int:1}, optString: { starts_with: "foo" }, AND: [{optBoolean: false, idTest: { ends_with: "5" }}]}){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id5"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id5"}]}}""")
  }

  "A filter query" should "support the AND filter in two recursion levels" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id4", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id5", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id6", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res =
      server.query(
        query = """{scalarModels(where: {AND: [{optBoolean: false, idTest: { ends_with: "5" }, AND: [{optString: { starts_with: "foo" }}]}]}){idTest}}""",
        project = project
      )
    val res2 =
      server.query(
        query =
          """{scalarModels(where: {b: {int:1}, AND: [{optBoolean: false, idTest: { ends_with: "5" }, AND: [{optString: { starts_with: "foo" }}]}]}){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id5"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id5"}]}}""")
  }

  "A filter query" should "support the OR filter in one recursion level" taggedAs (IgnoreMongo) in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id4", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id5", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res =
      server.query(
        query =
          """{scalarModels(where: {optBoolean: false, OR: [{optString: { starts_with: "foo" }}, {idTest: { ends_with: "5" }}]},orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"},{"idTest":"id4"},{"idTest":"id5"}]}}""")
  }

  "A filter query" should "support the OR filter in two recursion levels" taggedAs (IgnoreMongo) in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id4", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id5", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id6", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res =
      server.query(
        query =
          """{scalarModels(where: {OR: [{optString: { starts_with: "foo" }, OR: [{optBoolean: false},{idTest: { ends_with: "5" }}]}]},orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"},{"idTest":"id4"},{"idTest":"id5"},{"idTest":"id6"}]}}""")
  }
  //endregion

  //region null
  "A filter query" should "support filtering on null" in {
    createTest("id1", optString = null, 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", optString = "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", optString = null, 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val filterOnNull  = server.query(query = """{scalarModels(where: {optString: null} ,orderBy: { id: asc }){idTest}}""", project = project)
    val filterOnNull2 = server.query(query = """{scalarModels(where: {b: {int:1},optString: null},orderBy: { id: asc }){idTest}}""", project = project)

    filterOnNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    filterOnNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")

    val filterOnNotNull = server.query(query = """{scalarModels(where: {optString: { not: null }},orderBy: { id: asc }){idTest}}""", project = project)
    val filterOnNotNull2 =
      server.query(query = """{scalarModels(where: {b: {int:1},optString: { not: null }},orderBy: { id: asc }){idTest}}""", project = project)

    filterOnNotNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    filterOnNotNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")

    val filterOnInNull = server.query(query = """{scalarModels(where: {optString: { in: null }}, orderBy: { id: asc }){idTest}}""", project = project)
    val filterOnInNull2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optString: { in: null }},orderBy: { id: asc }){idTest}}""", project = project)

    filterOnInNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    filterOnInNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")

    val filterOnNotInNull = server.query(query = """{scalarModels(where: {optString: { not_in: null }},orderBy: { id: asc }){idTest}}""", project = project)
    val filterOnNotInNull2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optString: { not_in: null }},orderBy: { id: asc }){idTest}}""", project = project)

    filterOnNotInNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    filterOnNotInNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
  }

  //endregion

  //region String

  "A filter query" should "support the equality filter on strings" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: "bar"}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: "bar"}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not-equality filter on strings" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { not: "bar" }}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { not: "bar" }}, orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the contains filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: {contains: "bara" }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { contains: "bara" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not_contains filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query(query = """{scalarModels(where: {optString: { not_contains: "bara" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optString: { not_contains: "bara" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the starts_with filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { starts_with: "bar" }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { starts_with: "bar" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not_starts_with filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query(query = """{scalarModels(where: {optString: { not_starts_with: "bar" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optString: { not_starts_with: "bar" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the ends_with filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { ends_with: "bara" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { ends_with: "bara" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not_ends_with filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query(query = """{scalarModels(where: {optString: { not_ends_with: "bara" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optString: { not_ends_with: "bara" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { lt: "2" }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { lt: "2" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { lte: "2" },orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { lte: "2" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { gt: "2" }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { gt: "2" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optString: { gte: "2" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { gte: "2" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on strings" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val resA  = server.query(query = """{scalarModels(where: {optString: { in: ["a"] }}){idTest}}""", project = project)
    val resA2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { in: ["a"]}){idTest}}""", project = project)
    resA.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    resA2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")

    val resB  = server.query(query = """{scalarModels(where: {optString: { in: ["a","b"] }}){idTest}}""", project = project)
    val resB2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { in: ["a","b"] }}){idTest}}""", project = project)
    resB.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    resB2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")

    val resC  = server.query(query = """{scalarModels(where: {optString: { in: ["a","abc"] }},orderBy: { id: asc }){idTest}}""", project = project)
    val resC2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { in: ["a","abc"] }},orderBy: { id: asc }){idTest}}""", project = project)
    resC.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    resC2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")

    val resD  = server.query(query = """{scalarModels(where: {optString: { in: []}}){idTest}}""", project = project)
    val resD2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { in: []}}){idTest}}""", project = project)
    resD.toString() should be("""{"data":{"scalarModels":[]}}""")
    resD2.toString() should be("""{"data":{"scalarModels":[]}}""")
  }

  "A filter query" should "support the not_in filter on strings" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val resA  = server.query(query = """{scalarModels(where: {optString: { not_in: ["a"]}},orderBy: { id: asc }){idTest}}""", project = project)
    val resA2 = server.query(query = """{scalarModels(where: {b: {int:1}, optString: { not_in: ["a"]}},orderBy: { id: asc }){idTest}}""", project = project)
    resA.toString should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")

    val resB =
      server.query(query = """{scalarModels(orderBy: { idTest: asc }, where: {optString: { not_in: [] }},orderBy: { id: asc }){idTest}}""", project = project)
    val resB2 =
      server.query(query = """{scalarModels(orderBy: { idTest: asc }, where: {b: {int:1}, optString: { not_in: [] }},orderBy: { id: asc }){idTest}}""",
                   project = project)
    resB.toString should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"},{"idTest":"id3"}]}}""")
    resB2.toString should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region Integer

  "A filter query" should "support the equality filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: 1}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: 1}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not equality filter on integers" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { not: 1 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { not: 1 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { gt: 2 }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { gt: 2 }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on integers" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { in: [1] }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { in: [1] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not_in filter on integers" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optInt: { not_in: [1] }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optInt: { not_in: [1] }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region Float

  "A filter query" should "support the equality filter on float" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: 1}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: 1}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not equality filter on float" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { not: 1 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { not: 1 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { gt: 2 }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { gt: 2 }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on floats" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { in: [1] }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { in: [1] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not_in filter on floats" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optFloat: { not_in: [1] }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optFloat: { not_in: [1] }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  // region Boolean

  "A filter query" should "support the equality filter on booleans" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optBoolean: true}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optBoolean: true}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not-equality filter on booleans" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optBoolean: { not: true }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optBoolean: { not: true }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region DateTime

  "A filter query" should "support the equality filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optDateTime: "2016-09-24T12:29:32.342"}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: "2016-09-24T12:29:32.342"}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the not equality filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query(query = """{scalarModels(where: {optDateTime: { not: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: { not: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optDateTime: { lt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: { lt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query(query = """{scalarModels(where: {optDateTime: { lte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: { lte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optDateTime: { gt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: { gt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query(query = """{scalarModels(where: {optDateTime: { gte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: { gte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optDateTime: { in: ["2016-09-24T12:29:32.342Z"] }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optDateTime: { in: ["2016-09-24T12:29:32.342Z"] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the not_in filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query(query = """{scalarModels(where: {optDateTime: { not_in: ["2016-09-24T12:29:32.342Z"] }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query =
                              """{scalarModels(where: {b: {int:1}, optDateTime: { not_in: ["2016-09-24T12:29:32.342Z"] }},orderBy: { id: asc }){idTest}}""",
                            project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region Enum

  "A filter query" should "support the equality filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optEnum: A}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optEnum: A}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not equality filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optEnum: { not: A }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optEnum: { not: A }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optEnum: { in: [A] }}){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optEnum: { in: [A] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not in filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res  = server.query(query = """{scalarModels(where: {optEnum: { not_in: [A] }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(query = """{scalarModels(where: {b: {int:1}, optEnum: { not_in: [A] }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion
}
