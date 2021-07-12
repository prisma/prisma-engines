package writes.dataTypes.scalarLists

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsValue
import util.ConnectorCapability.ScalarListsCapability
import util._

// RS: Ported
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
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"createScalarModel":{"strings":["test${TroubleCharacters.value}"],"ints":[1337,12],"floats":[1.234,1.45],"decimals":["1.234","1.45"],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dA=="]}}}""".parseJson)

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
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"updateScalarModel":{"strings":["updated","now"],"ints":[14],"floats":[1.2345678],"decimals":["1.2345678"],"booleans":[false,false,true],"enums":[],"dateTimes":["2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA=="]}}}""".parseJson)

    res = server.query(
      s"""mutation {
         |  updateScalarModel(where: { id: 1 }, data: {
         |    strings:   { push: "future" }
         |    ints:      { push: 15 }
         |    floats:    { push: 2 }
         |    decimals:  { push: "2" }
         |    booleans:  { push: true }
         |    enums:     { push: A }
         |    dateTimes: { push: "2019-07-31T23:59:01.000Z" }
         |    bytes:     { push: "dGVzdA==" }
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"updateScalarModel":{"strings":["updated","now","future"],"ints":[14,15],"floats":[1.2345678,2],"decimals":["1.2345678","2"],"booleans":[false,false,true,true],"enums":["A"],"dateTimes":["2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dGVzdA=="]}}}""".parseJson)

    res = server.query(
      s"""mutation {
         |  updateScalarModel(where: { id: 1 }, data: {
         |    strings:   { push: ["more", "items"] }
         |    ints:      { push: [16, 17] }
         |    floats:    { push: [3, 4] }
         |    decimals:  { push: ["3", "4"] }
         |    booleans:  { push: [false, true] }
         |    enums:     { push: [B, A] }
         |    dateTimes: { push: ["2019-07-31T23:59:01.000Z", "2019-07-31T23:59:01.000Z"] }
         |    bytes:     { push: ["dGVzdA==", "dGVzdA=="] }
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"updateScalarModel":{"strings":["updated","now","future","more","items"],"ints":[14,15,16,17],"floats":[1.2345678,2.0,3.0,4.0],"decimals":["1.2345678","2","3","4"],"booleans":[false,false,true,true,false,true],"enums":["A","B","A"],"dateTimes":["2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z","2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dGVzdA==","dGVzdA==","dGVzdA=="]}}}""".parseJson)
  }

  "A Create Mutation" should "create and return items with list values with shorthand notation" in {
    val res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 1
         |    strings:   ["test${TroubleCharacters.value}"]
         |    ints:      [1337, 12]
         |    floats:    [1.234, 1.45]
         |    decimals:  ["1.234", "1.45"]
         |    booleans:  [true, false]
         |    enums:     [A,A]
         |    dateTimes: ["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"]
         |    bytes:     ["dGVzdA==", "dA=="]
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"createScalarModel":{"strings":["test${TroubleCharacters.value}"],"ints":[1337,12],"floats":[1.234,1.45],"decimals":["1.234","1.45"],"booleans":[true,false],"enums":["A","A"],"dateTimes":["2016-07-31T23:59:01.000Z","2017-07-31T23:59:01.000Z"],"bytes":["dGVzdA==","dA=="]}}}""".parseJson)
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
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res.toString() should be(
      """{"data":{"createScalarModel":{"strings":[],"ints":[],"floats":[],"decimals":[],"booleans":[],"enums":[],"dateTimes":[],"bytes":[]}}}""")
  }

  "A Create Mutation with an empty scalar list create input object" should "return a detailed error" in {
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

  "An Update Mutation with an empty scalar list update input object" should "return a detailed error" in {
    server.queryThatMustFail(
      s"""mutation {
         |  updateScalarModel(data: {
         |    id: 1
         |    strings: {},
         |  }){ strings, ints, floats, booleans, enums, dateTimes }
         |}""",
      project = project,
      errorCode = 2009,
      errorContains = """`Mutation.updateScalarModel.data.ScalarModelUpdateInput.strings.ScalarModelUpdatestringsInput`: Expected exactly one field to be present, got 0."""
    )
  }

  "An Update Mutation that pushes to some empty scalar lists" should "work" in {
    server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 1,
         |  }) {
         |    id
         |  }
         |}""",
      project = project
    )

    server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    id: 2,
         |  }) {
         |    id
         |  }
         |}""",
      project = project
    )

    var res = server.query(
      s"""mutation {
         |  updateScalarModel(where: { id: 1 }, data: {
         |    strings:   { push: "future" }
         |    ints:      { push: 15 }
         |    floats:    { push: 2 }
         |    decimals:  { push: "2" }
         |    booleans:  { push: true }
         |    enums:     { push: A }
         |    dateTimes: { push: "2019-07-31T23:59:01.000Z" }
         |    bytes:     { push: "dGVzdA==" }
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(
      s"""{"data":{"updateScalarModel":{"strings":["future"],"ints":[15],"floats":[2.0],"decimals":["2"],"booleans":[true],"enums":["A"],"dateTimes":["2019-07-31T23:59:01.000Z"],"bytes":["dGVzdA=="]}}}""".parseJson)

    res = server.query(
      s"""mutation {
         |  updateScalarModel(where: { id: 2 }, data: {
         |    strings:   { push: ["present", "future"] }
         |    ints:      { push: [14, 15] }
         |    floats:    { push: [1, 2] }
         |    decimals:  { push: ["1", "2"] }
         |    booleans:  { push: [false, true] }
         |    enums:     { push: [A, B] }
         |    dateTimes: { push: ["2019-07-31T23:59:01.000Z", "2019-07-31T23:59:02.000Z"] }
         |    bytes:     { push: ["dGVzdA==", "dGVzdA=="] }
         |  }) {
         |    strings
         |    ints
         |    floats
         |    decimals
         |    booleans
         |    enums
         |    dateTimes
         |    bytes
         |  }
         |}""",
      project = project
    )

    res should be(s"""{"data":{"updateScalarModel":{"strings":["present","future"],"ints":[14,15],"floats":[1.0,2.0],"decimals":["1","2"],"booleans":[false,true],"enums":["A","B"],"dateTimes":["2019-07-31T23:59:01.000Z","2019-07-31T23:59:02.000Z"],"bytes":["dGVzdA==","dGVzdA=="]}}}""".parseJson)
  }
}
