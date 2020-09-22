package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util._

class UpdateManySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = ProjectDsl.fromString {
    """model TestModel {
      |  id       String  @id @default(cuid())
      |  optStr   String?
      |  optInt   Int?
      |  optFloat Float?
      |}
    """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "An updateMany mutation" should "update the records matching the where clause" in {
    createTestModel("str1")
    createTestModel("str2")

    var result = server.query(
      """mutation {
        |  updateManyTestModel(
        |    where: { optStr: { equals: "str1" } }
        |    data: { optStr: { set: "str1new" }, optInt: { set: 1 }, optFloat: { multiply: 2 } }
        |  ) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsLong("data.updateManyTestModel.count") should equal(1)

    result = server.query(
      """{
        |  findManyTestModel(orderBy: { id: asc }) {
        |    optStr
        |    optInt
        |    optFloat
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.findManyTestModel").toString should be(
      """[{"optStr":"str1new","optInt":1,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]""")
  }

  "An updateMany mutation" should "update the records matching the where clause using shorthands" in {
    createTestModel("str1")
    createTestModel("str2")

    var result = server.query(
      """mutation {
        |  updateManyTestModel(
        |    where: { optStr: "str1" }
        |    data: { optStr: "str1new", optInt: null, optFloat: { multiply: 2 } }
        |  ) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsLong("data.updateManyTestModel.count") should equal(1)

    result = server.query(
      """{
        |  findManyTestModel(orderBy: { id: asc }) {
        |    optStr
        |    optInt
        |    optFloat
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.findManyTestModel").toString should be(
      """[{"optStr":"str1new","optInt":null,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]""")
  }

  "An updateMany mutation" should "update all items if the where clause is empty" in {
    createTestModel("str1")
    createTestModel("str2", Some(2))
    createTestModel("str3", Some(3), Some(3.1))

    var result = server.query(
      """mutation {
        |  updateManyTestModel(
        |    where: { }
        |    data: { optStr: { set: "updated" }, optFloat: { divide: 2 }, optInt: { decrement: 1 } }
        |  ){
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsLong("data.updateManyTestModel.count") should equal(3)

    result = server.query(
      """{
        |  findManyTestModel {
        |    optStr
        |    optInt
        |    optFloat
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.findManyTestModel").toString should be(
      """[{"optStr":"updated","optInt":null,"optFloat":null},{"optStr":"updated","optInt":1,"optFloat":null},{"optStr":"updated","optInt":2,"optFloat":1.55}]""")
  }

  "An updateMany mutation" should "correctly apply all number operations for Int" in {
    createTestModel("str1")
    createTestModel("str2", Some(2))
    createTestModel("str3", Some(3), Some(3.1))

    // Increment
    queryNumberOperation("optInt", "increment", "10") should be("""[{"optInt":null},{"optInt":12},{"optInt":13}]""")

    // Decrement
    queryNumberOperation("optInt", "decrement", "10") should be("""[{"optInt":null},{"optInt":2},{"optInt":3}]""")

    // Multiply
    queryNumberOperation("optInt", "multiply", "2") should be("""[{"optInt":null},{"optInt":4},{"optInt":6}]""")

    // Divide
    queryNumberOperation("optInt", "divide", "3") should be("""[{"optInt":null},{"optInt":1},{"optInt":2}]""")

    // Set
    queryNumberOperation("optInt", "set", "5") should be("""[{"optInt":5},{"optInt":5},{"optInt":5}]""")

    // Set null
    queryNumberOperation("optInt", "set", "null") should be("""[{"optInt":null},{"optInt":null},{"optInt":null}]""")
  }

  "An updateMany mutation" should "correctly apply all number operations for Float" in {
    createTestModel("str1")
    createTestModel("str2", None, Some(2))
    createTestModel("str3", None, Some(3.1))

    // Increment
    queryNumberOperation("optFloat", "increment", "1.1") should be("""[{"optFloat":null},{"optFloat":3.1},{"optFloat":4.2}]""")

    // Decrement
    queryNumberOperation("optFloat", "decrement", "1.1") should be("""[{"optFloat":null},{"optFloat":2},{"optFloat":3.1}]""")

    // Multiply
    queryNumberOperation("optFloat", "multiply", "5.5") should be("""[{"optFloat":null},{"optFloat":11},{"optFloat":17.05}]""")

    // Divide
    queryNumberOperation("optFloat", "divide", "2") should be("""[{"optFloat":null},{"optFloat":5.5},{"optFloat":8.525}]""")

    // Set
    queryNumberOperation("optFloat", "set", "5") should be("""[{"optFloat":5},{"optFloat":5},{"optFloat":5}]""")

    // Set null
    queryNumberOperation("optFloat", "set", "null") should be("""[{"optFloat":null},{"optFloat":null},{"optFloat":null}]""")
  }

  def queryNumberOperation(field: String, op: String, value: String): String = {
    var result = server.query(
      s"""mutation {
      |  updateManyTestModel(
      |    where: {}
      |    data: { $field: { $op: $value } }
      |  ){
      |    count
      |  }
      |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsLong("data.updateManyTestModel.count") should equal(3)

    result = server.query(
      s"""{
      |  findManyTestModel {
      |    $field
      |  }
      |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.findManyTestModel").toString
  }

  def createTestModel(optStr: String, optInt: Option[Int] = None, optFloat: Option[Double] = None): Unit = {
    val f = optFloat match {
      case Some(o) => s"$o"
      case None    => "null"
    }

    val i = optInt match {
      case Some(o) => s"$o"
      case None    => "null"
    }

    server.query(
      s"""
         |mutation {
         |  createOneTestModel(
         |    data: {
         |      optStr: "$optStr"
         |      optInt: $i
         |      optFloat: $f
         |    }
         |  ) {
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )
  }
}
