package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.PostgresConnectorTag
import util._

class ListFilterSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(PostgresConnectorTag)

  val project: Project = ProjectDsl.fromString { """
     |model Test {
     |  id        String    @id
     |  strList   String[]
     |  intList   Int[]
     |  floatList Float[]
     |  bIntList  BigInt[]
     |  decList   Decimal[]
     |  dtList    DateTime[]
     |  boolList  Boolean[]
     |  jsonList  Json[]
     |  bytesList Bytes[]
     |  enumList  TestEnum[]
     |}
     |
     |enum TestEnum {
     |  A
     |  B
     |}
     """.stripMargin }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    createTestData()
  }

  // equals
  // not equals
  // has
  // hasEvery
  // hasSome
  // isEmpty

  "The equals operation" should "work as expected" in {
    query("strList", "equals", """["a", "A", "c"]""", 1)
    query("intList", "equals", """[1, 2, 3]""", 1)
    query("floatList", "equals", """[1.1, 2.2, 3.3]""", 1)
    query("bIntList", "equals", """["100", "200", "300"]""", 1)
    query("decList", "equals", """["11.11", "22.22", "33.33"]""", 1)
    query("dtList", "equals", """["1969-01-01T10:33:59+00:00", "2018-12-05T12:34:23+00:00"]""", 1)
    query("boolList", "equals", """[true, false, false, true]""", 1)
    query("jsonList", "equals", """["{}", "{\"int\":5}", "[1, 2, 3]"]""", 1)
    query("bytesList", "equals", """["dGVzdA==", "dA=="]""", 1)
    query("enumList", "equals", """[A, B, B, A]""", 1)
  }

  "The has operation" should "work as expected" in {
    query("strList", "has", """"A"""", 1)
    query("intList", "has", """2""", 1)
    query("floatList", "has", """1.1""", 1)
    query("bIntList", "has", """"200"""", 1)
    query("decList", "has", """33.33""", 1)
    query("dtList", "has", """"2018-12-05T12:34:23+00:00"""", 1)
    query("boolList", "has", """true""", 1)
    query("jsonList", "has", """"[1, 2, 3]"""", 1)
    query("bytesList", "has", """"dGVzdA=="""", 1)
    query("enumList", "has", """A""", 1)
  }

  def query(field: String, operation: String, comparator: String, numExpectedRecords: Int): Unit = {
    val result = server.query(
      s"""
        |query {
        |  findManyTest(where: {
        |    $field: { $operation: $comparator }
        |  }) {
        |    id
        |  }
        |}
        |""".stripMargin,
      project,
      legacy = false
    )

    result.pathAsJsArray("data.findManyTest").value.length should be(numExpectedRecords)
  }

  def createTestData(): Unit = {
    server.query(
      s"""mutation {
        |createOneTest(data: {
        |  id:        "1",
        |  strList:   ["a", "A", "c"],
        |  intList:   [1, 2, 3],
        |  floatList: [1.1, 2.2, 3.3],
        |  bIntList:  ["100", "200", "300"],
        |  decList:   ["11.11", "22.22", "33.33"],
        |  dtList:    ["1969-01-01T10:33:59+00:00", "2018-12-05T12:34:23+00:00"],
        |  boolList:  [true, false, false, true],
        |  jsonList:  ["{}", "{\\"int\\":5}", "[1, 2, 3]"],
        |  bytesList: ["dGVzdA==", "dA=="],
        |  enumList:  [A, B, B, A]
        |}) { id }
        |}
        |""".stripMargin,
      project,
      legacy = false
    )
  }
}
