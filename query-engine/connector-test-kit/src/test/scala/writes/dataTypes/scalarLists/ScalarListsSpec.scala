package writes.dataTypes.scalarLists

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsValue
import util.ConnectorCapability.ScalarListsCapability
import util._

class ScalarListsSpec extends FlatSpec with Matchers with ApiSpecBase {

  override def runOnlyForCapabilities = Set(ScalarListsCapability)

  val project = ProjectDsl.fromString {
    s"""
      |model ScalarModel {
      |  id        Int @id
      |  strings   String[]
      |  ints      Int[]
      |  floats    Float[]
      |  decimals  Decimal[]
      |  booleans  Boolean[]
      |  enums     MyEnum[]
      |  dateTimes DateTime[]
      |  bytes     Bytes[]
      |  xmls      Xml[]
      |}
      |
      |enum MyEnum {
      |  A
      |  B
      |}
    """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "Scalar lists" should "be behave like regular values for create and update operations" in {
    var res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 1,
         |    strings:   { set: ["test${TroubleCharacters.value}"] }
         |    ints:      { set: [1337, 12] }
         |    floats:    { set: [1.234, 1.45] }
         |    decimals:  { set: ["1.234", "1.45"] }
         |    booleans:  { set: [true, false] }
         |    enums:     { set: [A, A] }
         |    dateTimes: { set: ["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"] }
         |    bytes:     { set: ["dGVzdA==", "dA=="] }
         |    xmls:      { set: ["<sense>none</sense>", "<cow>moo</cow>"] }
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |    xmls
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"createScalarModel":{"strings":["test${TroubleCharacters.value}"],"ints":[1337,12],"floats":[1.234,1.45],"decimals":["1.234","1.45"],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01+00:00","2017-07-31T23:59:01+00:00"],"bytes":["dGVzdA==","dA=="],"xmls":["<sense>none</sense>","<cow>moo</cow>"]}}}""".parseJson)

    res = server.query(
      s"""mutation {
         |  updateScalarModel(where: { id: 1 }, data: {
         |    strings:   { set: ["updated", "now"] }
         |    ints:      { set: [14] }
         |    floats:    { set: [1.2345678] }
         |    decimals:  { set: ["1.2345678"] }
         |    booleans:  { set: [false, false, true] }
         |    enums:     { set: [] }
         |    dateTimes: { set: ["2019-07-31T23:59:01.000Z"] }
         |    bytes:     { set: ["dGVzdA=="] }
         |    xmls:      { set: ["<chicken>cluck</chicken>"] }
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |    xmls
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"updateScalarModel":{"strings":["updated","now"],"ints":[14],"floats":[1.2345678],"decimals":["1.2345678"],"booleans":[false,false,true],"enums":[],"dateTimes":["2019-07-31T23:59:01+00:00"],"bytes":["dGVzdA=="],"xmls":["<chicken>cluck</chicken>"]}}}""".parseJson)
  }

  "A Create Mutation" should "create and return items with list values with shorthand notation" in {
    val res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 1
         |    strings:    ["test${TroubleCharacters.value}"]
         |    ints:      [1337, 12]
         |    floats:    [1.234, 1.45]
         |    decimals:  ["1.234", "1.45"]
         |    booleans:  [true, false]
         |    enums:     [A,A]
         |    dateTimes: ["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"]
         |    bytes:     ["dGVzdA==", "dA=="]
         |    xmls:      ["<sense>none</sense>", "<cow>moo</cow>"]
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |    xmls
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"createScalarModel":{"strings":["test${TroubleCharacters.value}"],"ints":[1337,12],"floats":[1.234,1.45],"decimals":["1.234","1.45"],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01+00:00","2017-07-31T23:59:01+00:00"],"bytes":["dGVzdA==","dA=="],"xmls":["<sense>none</sense>","<cow>moo</cow>"]}}}""".parseJson)
  }

  "A Create Mutation" should "create and return items with empty list values" in {
    val res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 1
         |    strings:   []
         |    ints:      []
         |    floats:    []
         |    decimals:  []
         |    booleans:  []
         |    enums:     []
         |    dateTimes: []
         |    bytes:     []
         |    xmls:      []
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |    xmls
         |  }
         |}""",
      project = project
    )

    res.toString() should be(
      """{"data":{"createScalarModel":{"strings":[],"ints":[],"floats":[],"decimals":[],"booleans":[],"enums":[],"dateTimes":[],"bytes":[],"xmls":[]}}}""")
  }

  "A Create Mutation with an empty scalar list update input object" should "return a detailed error" in {
    server.queryThatMustFail(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 1
         |    strings: {},
         |  }){ strings, ints, floats, booleans, enums, dateTimes }
         |}""",
      project = project,
      errorCode = 2009,
      errorContains = """`Mutation.createScalarModel.data.ScalarModelCreateInput.strings.ScalarModelCreatestringsInput.set`: A value is required but not set."""
    )
  }
}
