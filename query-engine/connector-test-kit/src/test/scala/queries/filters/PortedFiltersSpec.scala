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
      server.query(
        """{
          |  scalarModels(
          |    where: {
          |      optString: { startsWith: "foo" }
          |      optBoolean: { equals: false }
          |      idTest: { endsWith: "5" }
          |    }
          |  ) {
          |    idTest
          |  }
          |}""".stripMargin,
        project = project
      )

    val res2 =
      server.query(
        """{
          |  scalarModels(
          |    where: {
          |      b: { is: { int: { equals: 1 } } }
          |      AND: [
          |        { optString: { startsWith: "foo" }},
          |        { optBoolean: { equals: false }},
          |        { idTest: { endsWith: "5" }}
          |      ]
          |    }
          |  ) {
          |    idTest
          |  }
          |}
          |""",
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
        """
            |{
            |  scalarModels(
            |    where: {
            |      AND: [
            |        {
            |          optBoolean: { equals: false }
            |          idTest: { endsWith: "5" }
            |          AND: [{ optString: { startsWith: "foo" } }]
            |        }
            |      ]
            |    }
            |  ) {
            |    idTest
            |  }
            |}
          """.stripMargin,
        project = project
      )
    val res2 =
      server.query(
        """
            |{
            |  scalarModels(
            |    where: {
            |      b: { is: { int: { equals: 1 } } }
            |      AND: [
            |        {
            |          optBoolean: { equals: false }
            |          idTest: { endsWith: "5" }
            |          AND: [{ optString: { startsWith: "foo" } }]
            |        }
            |      ]
            |    }
            |  ) {
            |    idTest
            |  }
            |}
          """.stripMargin,
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
        """
            |{
            |  scalarModels(
            |    where: {
            |      optBoolean: { equals: false }
            |      OR: [
            |        { optString: { startsWith: "foo" } }
            |        { idTest: { endsWith: "5" } }
            |      ]
            |    }
            |    orderBy: { id: asc }
            |  ) {
            |    idTest
            |  }
            |}
          """.stripMargin,
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
        """
            |{
            |  scalarModels(
            |    where: {
            |      OR: [
            |        {
            |          optString: { startsWith: "foo" }
            |          OR: [
            |            { optBoolean: { equals: false } }
            |            { idTest: { endsWith: "5" } }
            |          ]
            |        }
            |      ]
            |    }
            |    orderBy: { id: asc }
            |  ) {
            |    idTest
            |  }
            |}
          """.stripMargin,
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

    val filterOnNull = server.query("""{scalarModels(where: { optString: { equals: null }}, orderBy: { id: asc }){ idTest }}""", project = project)
    val filterOnNull2 =
      server.query("""{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { equals: null }}, orderBy: { id: asc }){ idTest }}""",
                   project = project)

    filterOnNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    filterOnNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")

    val filterOnNotNull =
      server.query("""{scalarModels(where: { optString: { not: { equals: null }}}, orderBy: { id: asc }){ idTest }}""", project = project)

    val filterOnNotNullWithoutEquals =
      server.query("""{scalarModels(where: { optString: { not: null }}, orderBy: { id: asc }){ idTest }}""", project = project)

    // Must be the same as not null
    val filterOnNotNotNotNull =
      server.query("""{scalarModels(where: { optString: { not: { not: { not: { equals: null }}}}}, orderBy: { id: asc }){ idTest }}""", project = project)

    val filterOnNotNull2 =
      server.query(
        """{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { equals: null }}}, orderBy: { id: asc }){ idTest }}""",
        project = project
      )

    filterOnNotNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    filterOnNotNullWithoutEquals.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    filterOnNotNotNotNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    filterOnNotNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")

    val filterOnInNull = server.query("""{scalarModels(where: { optString: { in: null }}, orderBy: { id: asc }){idTest}}""", project = project)
    val filterOnInNull2 =
      server.query("""{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { in: null }}, orderBy: { id: asc }){ idTest }}""", project = project)

    filterOnInNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    filterOnInNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")

    val filterOnNotInNull = server.query("""{scalarModels(where: { optString: { not: { in: null }}}, orderBy: { id: asc }){ idTest }}""", project = project)
    val filterOnNotInNull2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { not: { in: null }}}, orderBy: { id: asc }){idTest}}""",
                   project = project)

    filterOnNotInNull.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    filterOnNotInNull2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
  }

  //endregion

  //region String

  "A filter query" should "support the equality filter on strings" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optString: { equals: "bar" }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { equals: "bar" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not-equality filter on strings" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: { optString: { not: { equals: "bar" }}}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { equals: "bar" }}}, orderBy: { id: asc }){idTest}}""",
                            project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the contains filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optString: {contains: "bara" }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { contains: "bara" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not contains filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optString: { not: { contains: "bara" }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { not: { contains: "bara" }}}, orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the startsWith filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: { optString: { startsWith: "bar" }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { startsWith: "bar" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not startsWith filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: { optString: { not: { startsWith: "bar" }}}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(
        """{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { not: { startsWith: "bar" }}}, orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the endsWith filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: { optString: { endsWith: "bara" }}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { endsWith: "bara" }}, orderBy: { id: asc }){idTest}}""",
                            project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not endsWith filter on strings" in {
    createTest("id1", "bara", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: { optString: { not: { endsWith: "bara" }}}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(
        """{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { not: { endsWith: "bara" }}}, orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: { optString: { lt: "2" }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: { b: { is: { int: { equals: 1 }}}, optString: { lt: "2" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: { optString: { lte: "2" }}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { lte: "2" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optString: { gt: "2" }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { gt: "2" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on strings" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optString: { gte: "2" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { gte: "2" }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on strings" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val resA  = server.query("""{scalarModels(where: {optString: { in: ["a"] }}){idTest}}""", project = project)
    val resA2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { in: ["a"]}}){idTest}}""", project = project)
    resA.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    resA2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")

    val resB  = server.query("""{scalarModels(where: {optString: { in: ["a","b"] }}){idTest}}""", project = project)
    val resB2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { in: ["a","b"] }}){idTest}}""", project = project)
    resB.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    resB2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")

    val resC = server.query("""{scalarModels(where: {optString: { in: ["a","abc"] }},orderBy: { id: asc }){idTest}}""", project = project)
    val resC2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { in: ["a","abc"] }},orderBy: { id: asc }){idTest}}""",
                             project = project)
    resC.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    resC2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")

    val resD  = server.query("""{scalarModels(where: {optString: { in: []}}){idTest}}""", project = project)
    val resD2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { in: []}}){idTest}}""", project = project)
    resD.toString() should be("""{"data":{"scalarModels":[]}}""")
    resD2.toString() should be("""{"data":{"scalarModels":[]}}""")
  }

  "A filter query" should "support the not in filter on strings" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val resA = server.query("""{scalarModels(where: {optString: { not: { in: ["a"] }}}, orderBy: { id: asc }){idTest}}""", project = project)
    val resA2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optString: { not: { in: ["a"] }}},orderBy: { id: asc }){idTest}}""",
                   project = project)
    resA.toString should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")

    val resB =
      server.query("""{scalarModels(orderBy: { idTest: asc }, where: {optString: { not: { in: [] }}},orderBy: { id: asc }){idTest}}""", project = project)
    val resB2 =
      server.query(
        """{scalarModels(orderBy: { idTest: asc }, where: {b: { is: { int: { equals: 1 }}}, optString: { not: { in: [] }}},orderBy: { id: asc }){idTest}}""",
        project = project
      )
    resB.toString should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"},{"idTest":"id3"}]}}""")
    resB2.toString should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region Integer

  "A filter query" should "support the equality filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: { optInt: { equals: 1 }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { equals: 1 }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not equality filter on integers" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optInt: { not: { equals: 1 }}}, orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { not: { equals: 1 }}},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optInt: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optInt: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optInt: { gt: 2 }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { gt: 2 }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on integers" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optInt: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on integers" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optInt: { in: [1] }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { in: [1] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not in filter on integers" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optInt: { not: { in: [1] }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optInt: { not: { in: [1] }}},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region Float

  "A filter query" should "support the equality filter on float" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optFloat: { equals: 1 }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { equals: 1 }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not equality filter on float" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optFloat: { not: { equals: 1 }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { not: { equals: 1 }}},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optFloat: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { lt: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optFloat: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { lte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optFloat: { gt: 2 }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { gt: 2 }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on floats" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optFloat: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { gte: 2 }},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on floats" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optFloat: { in: [1] }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { in: [1] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not in filter on floats" in {
    createTest("id1", "a", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "ab", 2, 2, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "abc", 3, 3, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optFloat: { not: { in: [1] }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optFloat: { not: { in: [1] }}},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  // region Boolean

  "A filter query" should "support the equality filter on booleans" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optBoolean: { equals: true }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optBoolean: { equals: true }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not-equality filter on booleans" in {
    createTest("id1", "bar", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "foo bar", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")
    createTest("id3", "foo bar barz", 1, 1, optBoolean = false, "A", "2016-09-23T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optBoolean: { not: { equals: true }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optBoolean: { not: { equals: true }}},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region DateTime

  "A filter query" should "support the equality filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optDateTime: { equals: "2016-09-24T12:29:32.342" }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { equals: "2016-09-24T12:29:32.342" }}){idTest}}""",
                            project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the not equality filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query("""{scalarModels(where: {optDateTime: { not: { equals: "2016-09-24T12:29:32.342Z" }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(
        """{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { not: { equals: "2016-09-24T12:29:32.342Z" }}},orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the lt filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optDateTime: { lt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { lt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the lte filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query("""{scalarModels(where: {optDateTime: { lte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(
        """{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { lte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the gt filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optDateTime: { gt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { gt: "2016-09-24T12:29:32.342Z" }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the gte filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query("""{scalarModels(where: {optDateTime: { gte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query(
        """{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { gte: "2016-09-24T12:29:32.342Z" }},orderBy: { id: asc }){idTest}}""",
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optDateTime: { in: ["2016-09-24T12:29:32.342Z"] }}){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { in: ["2016-09-24T12:29:32.342Z"] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
  }

  "A filter query" should "support the not in filter on DateTime" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "A", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "A", "2016-09-25T12:29:32.342")

    val res =
      server.query("""{scalarModels(where: {optDateTime: { not: { in: ["2016-09-24T12:29:32.342Z"] }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 = server.query(
      query =
        """{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optDateTime: { not: { in: ["2016-09-24T12:29:32.342Z"] }}},orderBy: { id: asc }){idTest}}""",
      project = project
    )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"},{"idTest":"id3"}]}}""")
  }
  //endregion

  //region Enum

  "A filter query" should "support the equality filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optEnum: { equals: A }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optEnum: { equals: A }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not equality filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optEnum: { not: { equals: A }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optEnum: { not: { equals: A }}},orderBy: { id: asc }){idTest}}""",
                   project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }

  "A filter query" should "support the in filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res  = server.query("""{scalarModels(where: {optEnum: { in: [A] }}){idTest}}""", project = project)
    val res2 = server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optEnum: { in: [A] }}){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id1"}]}}""")
  }

  "A filter query" should "support the not in filter on Enum" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res = server.query("""{scalarModels(where: {optEnum: { not: { in: [A] }}},orderBy: { id: asc }){idTest}}""", project = project)
    val res2 =
      server.query("""{scalarModels(where: {b: { is: { int: { equals: 1 }}}, optEnum: { not: { in: [A] }}},orderBy: { id: asc }){idTest}}""", project = project)

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
    res2.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"},{"idTest":"id3"}]}}""")
  }
  //endregion

  "A filter query" should "should treat NOT and not filters equally" in {
    createTest("id1", "1", 1, 1, optBoolean = true, "A", "2016-09-23T12:29:32.342")
    createTest("id2", "2", 2, 2, optBoolean = false, "B", "2016-09-24T12:29:32.342")
    createTest("id3", "3", 3, 3, optBoolean = false, "B", "2016-09-25T12:29:32.342")

    val res = server.query(
      """
        |{
        |  scalarModels(
        |    where: { optString: { not: { equals: "1", gt: "2" } } }
        |    orderBy: { id: asc }
        |  ) {
        |    idTest
        |  }
        |}
      """.stripMargin,
      project = project
    )

    val res2 =
      server.query(
        """
          |{
          |  scalarModels(
          |    where: { NOT: [
          |      { optString: { equals: "1" }},
          |      { optString: { gt: "2" }}
          |    ]}
          |    orderBy: { id: asc }
          |  ) {
          |    idTest
          |  }
          |}
        """.stripMargin,
        project = project
      )

    res.toString() should be("""{"data":{"scalarModels":[{"idTest":"id2"}]}}""")
    res.toString() should be(res2.toString)
  }
}
