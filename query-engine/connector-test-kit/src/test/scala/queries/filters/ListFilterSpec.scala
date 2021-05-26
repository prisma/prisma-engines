package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.PostgresConnectorTag
import util._

// RS: Ported
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

  "The `equals` operation" should "work as expected" in {
    query("strList", "equals", """["a", "A", "c"]""", Some(1))
    query("intList", "equals", """[1, 2, 3]""", Some(1))
    query("floatList", "equals", """[1.1, 2.2, 3.3]""", Some(1))
    query("bIntList", "equals", """["100", "200", "300"]""", Some(1))
    query("decList", "equals", """["11.11", "22.22", "33.33"]""", Some(1))
    query("dtList", "equals", """["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"]""", Some(1))
    query("boolList", "equals", """[true]""", Some(1))
    query("jsonList", "equals", """["{}", "{\"int\":5}", "[1, 2, 3]"]""", Some(1))
    query("bytesList", "equals", """["dGVzdA==", "dA=="]""", Some(1))
    query("enumList", "equals", """[A, B, B, A]""", Some(1))
  }

  "The `has` operation" should "work as expected" in {
    query("strList", "has", """"A"""", Some(1))
    query("intList", "has", """2""", Some(1))
    query("floatList", "has", """1.1""", Some(1))
    query("bIntList", "has", """"200"""", Some(1))
    query("decList", "has", """33.33""", Some(1))
    query("dtList", "has", """"2018-12-05T12:34:23.000Z"""", Some(1))
    query("boolList", "has", """true""", Some(1))
    query("jsonList", "has", """"[1, 2, 3]"""", Some(1))
    query("bytesList", "has", """"dGVzdA=="""", Some(1))
    query("enumList", "has", """A""", Some(1))
  }

  "The `hasSome` operation" should "work as expected" in {
    query("strList", "hasSome", """["A", "c"]""", Some(1))
    query("intList", "hasSome", """[2, 10]""", Some(1))
    query("floatList", "hasSome", """[1.1, 5.5]""", Some(1))
    query("bIntList", "hasSome", """["200", "5000"]""", Some(1))
    query("decList", "hasSome", """[55.55, 33.33]""", Some(1))
    query("dtList", "hasSome", """["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]""", Some(1))
    query("boolList", "hasSome", """[true, false]""", Some(1))
    query("jsonList", "hasSome", """["{}", "[1]"]""", Some(1))
    query("bytesList", "hasSome", """["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]""", Some(1))
    query("enumList", "hasSome", """[A]""", Some(1))

    query("strList", "hasSome", """[]""", None)
  }

  "The `hasEvery` operation" should "work as expected" in {
    query("strList", "hasEvery", """["A", "d"]""", None)
    query("strList", "hasEvery", """["A"]""", Some(1))

    query("intList", "hasEvery", """[2, 10]""", None)
    query("intList", "hasEvery", """[2]""", Some(1))

    query("floatList", "hasEvery", """[1.1, 5.5]""", None)
    query("floatList", "hasEvery", """[1.1]""", Some(1))

    query("bIntList", "hasEvery", """["200", "5000"]""", None)
    query("bIntList", "hasEvery", """["200"]""", Some(1))

    query("decList", "hasEvery", """[55.55, 33.33]""", None)
    query("decList", "hasEvery", """[33.33]""", Some(1))

    query("dtList", "hasEvery", """["2018-12-05T12:34:23.000Z", "2019-12-05T12:34:23.000Z"]""", None)
    query("dtList", "hasEvery", """["2018-12-05T12:34:23.000Z"]""", Some(1))

    query("boolList", "hasEvery", """[true, false]""", None)
    query("boolList", "hasEvery", """[true]""", Some(1))

    query("jsonList", "hasEvery", """["{}", "[1]"]""", None)
    query("jsonList", "hasEvery", """["{}"]""", Some(1))

    query("bytesList", "hasEvery", """["dGVzdA==", "bG9va2luZyBmb3Igc29tZXRoaW5nPw=="]""", None)
    query("bytesList", "hasEvery", """["dGVzdA=="]""", Some(1))

    query("enumList", "hasEvery", """[A, B]""", Some(1))
  }

  "Querying `hasEvery` with an empty input" should "return all" in {
    val result = server.query(
      s"""
         |query {
         |  findManyTest(where: {
         |    strList: { hasEvery: [] }
         |  }) {
         |    id
         |  }
         |}
         |""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyTest":[{"id":"1"},{"id":"2"}]}}""")
  }

  "The `isEmpty` operation" should "work as expected" in {
    query("strList", "isEmpty", "true", Some(2))
    query("intList", "isEmpty", "true", Some(2))
    query("floatList", "isEmpty", "true", Some(2))
    query("bIntList", "isEmpty", "true", Some(2))
    query("decList", "isEmpty", "true", Some(2))
    query("dtList", "isEmpty", "true", Some(2))
    query("boolList", "isEmpty", "true", Some(2))
    query("jsonList", "isEmpty", "true", Some(2))
    query("bytesList", "isEmpty", "true", Some(2))
    query("enumList", "isEmpty", "true", Some(2))

    query("strList", "isEmpty", "false", Some(1))
    query("intList", "isEmpty", "false", Some(1))
    query("floatList", "isEmpty", "false", Some(1))
    query("bIntList", "isEmpty", "false", Some(1))
    query("decList", "isEmpty", "false", Some(1))
    query("dtList", "isEmpty", "false", Some(1))
    query("boolList", "isEmpty", "false", Some(1))
    query("jsonList", "isEmpty", "false", Some(1))
    query("bytesList", "isEmpty", "false", Some(1))
    query("enumList", "isEmpty", "false", Some(1))
  }

  def query(field: String, operation: String, comparator: String, expectedId: Option[Int]): Unit = {
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

    expectedId match {
      case Some(id) => result.toString() should be(s"""{"data":{"findManyTest":[{"id":"$id"}]}}""")
      case None     => result.pathAsSeq("data.findManyTest").length should be(0)
    }
  }

  // 1 with full data
  // 1 empty
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
        |  dtList:    ["1969-01-01T10:33:59.000Z", "2018-12-05T12:34:23.000Z"],
        |  boolList:  [true],
        |  jsonList:  ["{}", "{\\"int\\":5}", "[1, 2, 3]"],
        |  bytesList: ["dGVzdA==", "dA=="],
        |  enumList:  [A, B, B, A]
        |}) { id }
        |}
        |""".stripMargin,
      project,
      legacy = false
    )

    server.query(
      s"""mutation {
         |createOneTest(data: {
         |  id:        "2",
         |  strList:   [],
         |  intList:   [],
         |  floatList: [],
         |  bIntList:  [],
         |  decList:   [],
         |  dtList:    [],
         |  boolList:  [],
         |  jsonList:  [],
         |  bytesList: [],
         |  enumList:  []
         |}) { id }
         |}
         |""".stripMargin,
      project,
      legacy = false
    )
  }
}
