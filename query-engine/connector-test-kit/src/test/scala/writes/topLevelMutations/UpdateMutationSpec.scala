package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.Json
import util._

class UpdateMutationSpec extends FlatSpec with Matchers with ApiSpecBase {
  "An updateOne mutation" should "update an item" taggedAs IgnoreSQLite in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id          String  @id @default(cuid())
        |  optString   String?
        |  optInt      Int?
        |  optFloat    Float?
        |  optBoolean  Boolean?
        |  optDateTime DateTime?
        |}
      """.stripMargin
    }
    database.setup(project)

    val createResult = server.query(
      """
        |mutation {
        |  createOneTestModel(data: {}) {
        |    id
        |  }
        |}
      """.stripMargin,
      project = project,
      legacy = false,
    )

    val id = createResult.pathAsString("data.createOneTestModel.id")
    val updateResult = server.query(
      s"""
        |mutation {
        |  updateOneTestModel(
        |    where: { id: "$id" }
        |    data: {
        |      optString: { set: "test${TroubleCharacters.value}" }
        |      optInt: { set: 1337 }
        |      optFloat: { set: 1.234 }
        |      optBoolean: { set: true }
        |      optDateTime: { set: "2016-07-31T23:59:01.000Z" }
        |    }
        |  ) {
        |    optString
        |    optInt
        |    optFloat
        |    optBoolean
        |    optDateTime
        |  }
        |}
        |
      """.stripMargin,
      project,
      legacy = false,
    )

    updateResult.pathAsJsValue("data.updateOneTestModel") should be(
      Json.parse(
        s"""{"optString":"test${TroubleCharacters.value}","optInt":1337,"optFloat":1.234,"optBoolean":true,"optDateTime":"2016-07-31T23:59:01.000Z"}"""))

    val readResult = server.query(
      s"""
         |{
         |  findManyTestModel {
         |    id
         |  }
         |}
       """.stripMargin,
      project,
      legacy = false,
    )

    readResult.pathAsJsValue("data.findManyTestModel").toString should equal(s"""[{"id":"$id"}]""")
  }

  "An updateOne mutation" should "update an item with shorthand notation" taggedAs IgnoreSQLite in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id          String  @id @default(cuid())
        |  optString   String?
        |  optInt      Int?
        |  optFloat    Float?
        |  optBoolean  Boolean?
        |  optDateTime DateTime?
        |}
      """.stripMargin
    }
    database.setup(project)

    val createResult = server.query(
      """
        |mutation {
        |  createOneTestModel(data: {}) {
        |    id
        |  }
        |}
      """.stripMargin,
      project = project,
      legacy = false,
    )

    val id = createResult.pathAsString("data.createOneTestModel.id")
    val updateResult = server.query(
      s"""
         |mutation {
         |  updateOneTestModel(
         |    where: { id: "$id" }
         |    data: {
         |      optString: "test${TroubleCharacters.value}",
         |      optInt: 1337,
         |      optFloat: 1.234,
         |      optBoolean: true,
         |      optDateTime: "2016-07-31T23:59:01.000Z",
         |    }
         |  ) {
         |    optString
         |    optInt
         |    optFloat
         |    optBoolean
         |    optDateTime
         |  }
         |}
         |
      """.stripMargin,
      project,
      legacy = false,
    )

    updateResult.pathAsJsValue("data.updateOneTestModel") should be(
      Json.parse(
        s"""{"optString":"test${TroubleCharacters.value}","optInt":1337,"optFloat":1.234,"optBoolean":true,"optDateTime":"2016-07-31T23:59:01.000Z"}"""))

    val readResult = server.query(
      s"""
         |{
         |  findManyTestModel {
         |    id
         |  }
         |}
       """.stripMargin,
      project,
      legacy = false,
    )

    readResult.pathAsJsValue("data.findManyTestModel").toString should equal(s"""[{"id":"$id"}]""")
  }

  "An updateOne mutation" should "update an item by a unique field" in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id        String  @id @default(cuid())
        |  strField  String
        |  uniqField String? @unique
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      s"""
        |mutation {
        |  createOneTestModel(
        |    data: {
        |      strField: "test"
        |      uniqField: "uniq"
        |    }
        |  ){
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    val updateResult = server.query(
      s"""
        |mutation {
        |  updateOneTestModel(
        |    where: { uniqField: "uniq" }
        |    data: { strField: { set: "updated" } }
        |  ){
        |    strField
        |  }
        |}""".stripMargin,
      project,
      legacy = false,
    )
    updateResult.pathAsString("data.updateOneTestModel.strField") should equal("updated")
  }

  "An updateOne mutation" should "update enums" taggedAs IgnoreSQLite in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id      Int     @id
        |  optEnum MyEnum?
        |}
        |
        |enum MyEnum {
        |  A
        |  ABCD
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """
        |mutation {
        |  createOneTestModel(data: { id: 1 }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project = project,
      legacy = false,
    )

    val updateResult = server.query(
      s"""
         |mutation {
         |  updateOneTestModel(
         |    where: { id: 1 }
         |    data: { optEnum: { set: A } }
         |  ) {
         |    optEnum
         |  }
         |}
         |
      """.stripMargin,
      project,
      legacy = false,
    )

    updateResult.pathAsJsValue("data.updateOneTestModel") should be(Json.parse("""{"optEnum":"A"}"""))
  }

  "An updateOne mutation" should "gracefully fail when trying to update an item by a unique field with a non-existing value" in {
    val project = ProjectDsl.fromString {
      """
          |model TestModel {
          |  id        String  @id @default(cuid())
          |  strField  String
          |  uniqField String? @unique
          |}
        """.stripMargin
    }
    database.setup(project)

    server.query(
      s"""
         |mutation {
         |  createOneTestModel(
         |    data: {
         |      strField: "test", uniqField: "uniq"
         |    }
         |  ){
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    server.queryThatMustFail(
      s"""
         |mutation {
         |  updateOneTestModel(
         |    where: { uniqField: "doesn't exist" }
         |    data: { strField: { set: "updated" } }
         |  ){
         |    id
         |  }
         |}""".stripMargin,
      project,
      legacy = false,
      errorCode = 2016,
      errorContains = """Query interpretation error. Error for binding '0': RecordNotFound(\"Record to update not found.\"""
    )
  }

  "An updateOne mutation" should "update an updatedAt datetime" in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id        Int      @id
        |  field     String
        |  updatedAt DateTime @updatedAt
        |  createdAt DateTime @default(now())
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """
         |mutation {
         |  createOneTestModel(data: { id: 1, field: "test" }){
         |    id
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    Thread.sleep(1000)

    val res = server.query(
      """
         |mutation {
         |  updateOneTestModel(
         |    where: { id: 1 }
         |    data: { field: { set: "test2" } }
         |  ){
         |    createdAt
         |    updatedAt
         |  }
         |}""",
      project,
      legacy = false,
    )

    val createdAt = res.pathAsString("data.updateOneTestModel.createdAt")
    val updatedAt = res.pathAsString("data.updateOneTestModel.updatedAt")

    createdAt should not be updatedAt
  }

  "UpdatedAt and createdAt" should "be mutable with an update" in {
    val project = ProjectDsl.fromString {
      """
        |model TestModel {
        |  id        Int      @id
        |  createdAt DateTime @default(now())
        |  updatedAt DateTime @updatedAt
        |}
      """.stripMargin
    }
    database.setup(project)

    server
      .query(
        s"""mutation {
         |  createOneTestModel(
         |    data:{
         |      id: 1
         |    }
         |  ){
         |    id
         |  }
         |}""".stripMargin,
        project = project,
        legacy = false,
      )

    val res = server
      .query(
        s"""
           |mutation {
           |  updateOneTestModel(
           |    where: { id: 1 }
           |    data: {
           |      createdAt: { set: "2000-01-01T00:00:00Z" }
           |      updatedAt: { set: "2001-01-01T00:00:00Z" }
           |    }
           |  ) {
           |    createdAt
           |    updatedAt
           |  }
           |}
         """.stripMargin,
        project = project,
        legacy = false,
      )

    // We currently have a datetime precision of 3, so Prisma will add .000
    res.pathAsString("data.updateOneTestModel.createdAt") should be("2000-01-01T00:00:00.000Z")
    res.pathAsString("data.updateOneTestModel.updatedAt") should be("2001-01-01T00:00:00.000Z")
  }

  "An updateOne mutation" should "correctly apply all number operations for Int" in {
    val project = ProjectDsl.fromString {
      """model TestModel {
        |  id       Int     @id
        |  optInt   Int?
        |  optFloat Float?
        |}
      """.stripMargin
    }

    database.setup(project)
    createTestModel(project, 1)
    createTestModel(project, 2, Some(3))

    // Increment
    queryNumberOperation(project, 1, "optInt", "increment", "10") should be("""{"optInt":null}""")
    queryNumberOperation(project, 2, "optInt", "increment", "10") should be("""{"optInt":13}""")

    // Decrement
    queryNumberOperation(project, 1, "optInt", "decrement", "10") should be("""{"optInt":null}""")
    queryNumberOperation(project, 2, "optInt", "decrement", "10") should be("""{"optInt":3}""")

    // Multiply
    queryNumberOperation(project, 1, "optInt", "multiply", "2") should be("""{"optInt":null}""")
    queryNumberOperation(project, 2, "optInt", "multiply", "2") should be("""{"optInt":6}""")

    // Divide
    queryNumberOperation(project, 1, "optInt", "divide", "3") should be("""{"optInt":null}""")
    queryNumberOperation(project, 2, "optInt", "divide", "3") should be("""{"optInt":2}""")

    // Set
    queryNumberOperation(project, 1, "optInt", "set", "5") should be("""{"optInt":5}""")
    queryNumberOperation(project, 2, "optInt", "set", "5") should be("""{"optInt":5}""")

    // Set null
    queryNumberOperation(project, 1, "optInt", "set", "null") should be("""{"optInt":null}""")
    queryNumberOperation(project, 2, "optInt", "set", "null") should be("""{"optInt":null}""")
  }

  "An updateOne mutation" should "correctly apply all number operations for Float" in {
    val project = ProjectDsl.fromString {
      """model TestModel {
          id        Int     @id
        |  optInt   Int?
        |  optFloat Float?
        |}
      """.stripMargin
    }

    database.setup(project)
    createTestModel(project, 1)
    createTestModel(project, 2, None, Some(5.5))

    // Increment
    queryNumberOperation(project, 1, "optFloat", "increment", "4.6") should be("""{"optFloat":null}""")
    queryNumberOperation(project, 2, "optFloat", "increment", "4.6") should be("""{"optFloat":10.1}""")

    // Decrement
    queryNumberOperation(project, 1, "optFloat", "decrement", "4.6") should be("""{"optFloat":null}""")
    queryNumberOperation(project, 2, "optFloat", "decrement", "4.6") should be("""{"optFloat":5.5}""")

    // Multiply
    queryNumberOperation(project, 1, "optFloat", "multiply", "2") should be("""{"optFloat":null}""")
    queryNumberOperation(project, 2, "optFloat", "multiply", "2") should be("""{"optFloat":11}""")

    // Divide
    queryNumberOperation(project, 1, "optFloat", "divide", "2") should be("""{"optFloat":null}""")
    queryNumberOperation(project, 2, "optFloat", "divide", "2") should be("""{"optFloat":5.5}""")

    // Set
    queryNumberOperation(project, 1, "optFloat", "set", "5.1") should be("""{"optFloat":5.1}""")
    queryNumberOperation(project, 2, "optFloat", "set", "5.1") should be("""{"optFloat":5.1}""")

    // Set null
    queryNumberOperation(project, 1, "optFloat", "set", "null") should be("""{"optFloat":null}""")
    queryNumberOperation(project, 2, "optFloat", "set", "null") should be("""{"optFloat":null}""")
  }

  "An updateOne mutation with number operations" should "handle id changes correctly" in {
    val project = ProjectDsl.fromString {
      """model TestModel {
        |  id1  Float
        |  id2  Int
        |  uniq Int @unique
        |  
        |  @@id([id1, id2])
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      s"""
         |mutation {
         |  createOneTestModel(
         |    data: {
         |      id1: 1.23456
         |      id2: 2
         |      uniq: 3
         |    }
         |  ) {
         |    uniq
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    val result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { id1_id2: { id1: 1.23456, id2: 2 } }
         |    data: {
         |      id1: { divide: 2 }
         |      uniq: { multiply: 3 }
         |    }
         |  ){
         |    id1
         |    id2
         |    uniq
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel").toString should be("""{"id1":0.61728,"id2":2,"uniq":9}""")
  }

  def queryNumberOperation(project: Project, id: Int, field: String, op: String, value: String): String = {
    val result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { id: $id }
         |    data: { $field: { $op: $value } }
         |  ){
         |    $field
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel").toString
  }

  def createTestModel(project: Project, id: Int, optInt: Option[Int] = None, optFloat: Option[Double] = None): Unit = {
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
         |      id: $id
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
