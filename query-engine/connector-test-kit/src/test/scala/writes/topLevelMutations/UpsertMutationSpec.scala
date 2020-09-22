package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util._

class UpsertMutationSpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = ProjectDsl.fromString {
    """
      |model Todo {
      |  id             String @id @default(cuid())
      |  title          String
      |  alias          String  @unique
      |  anotherIDField String? @unique
      |}
      |
      |model WithDefaultValue {
      |  id        String @id @default(cuid())
      |  reqString String @default(value: "defaultValue")
      |  title     String
      |}
      |
      |model MultipleFields {
      |  id         String @id @default(cuid())
      |  reqString  String
      |  reqInt     Int
      |  reqFloat   Float
      |  reqBoolean Boolean
      |}
    """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "an item" should "be created if it does not exist yet" in {
    todoCount should be(0)

    val todoId = "5beea4aa6183dd734b2dbd9b"
    val result = server.query(
      s"""mutation {
        |  upsertOneTodo(
        |    where: {id: "$todoId"}
        |    create: {
        |      title: "new title"
        |      alias: "todo1"
        |    }
        |    update: {
        |      title: { set: "updated title" }
        |    }
        |  ){
        |    id
        |    title
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should be("new title")

    todoCount should be(1)
  }

  "an item" should "be created with multiple fields of different types" in {
    todoCount should be(0)

    val id = "5beea4aa6183dd734b2dbd9b"
    val result = server.query(
      s"""mutation {
         |  upsertOneMultipleFields(
         |    where: {id: "$id"}
         |    create: {
         |      reqString: "new title"
         |      reqInt: 1
         |      reqFloat: 1.22
         |      reqBoolean: true
         |    }
         |    update: {
         |      reqString: { set: "title" }
         |      reqInt: { set: 2 }
         |      reqFloat: { set: 5.223423423423 }
         |      reqBoolean: { set: false }
         |    }
         |  ){
         |    id
         |    reqString
         |    reqInt
         |    reqFloat
         |    reqBoolean
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneMultipleFields.reqString") should be("new title")
    result.pathAsLong("data.upsertOneMultipleFields.reqInt") should be(1)
    result.pathAsDouble("data.upsertOneMultipleFields.reqFloat") should be(1.22)
    result.pathAsBool("data.upsertOneMultipleFields.reqBoolean") should be(true)

  }

  "an item" should "be created if it does not exist yet and use the defaultValue if necessary" in {
    val todoId = "5beea4aa6183dd734b2dbd9b"
    val result = server.query(
      s"""mutation {
         |  upsertOneWithDefaultValue(
         |    where: {id: "$todoId"}
         |    create: {
         |      title: "new title"
         |    }
         |    update: {
         |      title: { set: "updated title" }
         |    }
         |  ){
         |    title
         |    reqString
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneWithDefaultValue.title") should be("new title")
    result.pathAsString("data.upsertOneWithDefaultValue.reqString") should be("defaultValue")
  }

  "an item" should "not be created when trying to set a required value to null even if there is a default value for that field" in {
    server.queryThatMustFail(
      s"""mutation {
         |  upsertOneWithDefaultValue(
         |    where: {id: "NonExistantID"}
         |    create: {
         |      reqString: null
         |      title: "new title"
         |    }
         |    update: {
         |      title: { set: "updated title" }
         |    }
         |  ){
         |    title
         |    reqString
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
      errorCode = 2012,
      errorContains = "Missing a required value at `Mutation.upsertOneWithDefaultValue.create.WithDefaultValueCreateInput.reqString`"
    )
  }

  "an item" should "be updated if it already exists (by id)" in {
    val todoId = server
      .query(
        """mutation {
        |  createOneTodo(
        |    data: {
        |      title: "new title1"
        |      alias: "todo1"
        |    }
        |  ) {
        |    id
        |  }
        |}
      """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.createOneTodo.id")

    todoCount should be(1)

    val result = server.query(
      s"""mutation {
         |  upsertOneTodo(
         |    where: {id: "$todoId"}
         |    create: {
         |      title: "irrelevant"
         |      alias: "irrelevant"
         |    }
         |    update: {
         |      title: { set: "updated title" }
         |    }
         |  ){
         |    id
         |    title
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should be("updated title")

    todoCount should be(1)
  }

  "an item" should "be updated using shorthands if it already exists (by id)" in {
    val todoId = server
      .query(
        """mutation {
          |  createOneTodo(
          |    data: {
          |      title: "new title1"
          |      alias: "todo1"
          |    }
          |  ) {
          |    id
          |  }
          |}
        """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.createOneTodo.id")

    todoCount should be(1)

    val result = server.query(
      s"""mutation {
         |  upsertOneTodo(
         |    where: {id: "$todoId"}
         |    create: {
         |      title: "irrelevant"
         |      alias: "irrelevant"
         |    }
         |    update: {
         |      title: "updated title"
         |    }
         |  ){
         |    id
         |    title
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should be("updated title")

    todoCount should be(1)
  }

  "an item" should "be updated if it already exists (by any unique argument)" in {
    val todoAlias = server
      .query(
        """mutation {
          |  createOneTodo(
          |    data: {
          |      title: "new title1"
          |      alias: "todo1"
          |    }
          |  ) {
          |    alias
          |  }
          |}
        """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.createOneTodo.alias")

    todoCount should be(1)

    val result = server.query(
      s"""mutation {
         |  upsertOneTodo(
         |    where: {alias: "$todoAlias"}
         |    create: {
         |      title: "irrelevant"
         |      alias: "irrelevant"
         |    }
         |    update: {
         |      title: { set:"updated title" }
         |    }
         |  ){
         |    id
         |    title
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should be("updated title")

    todoCount should be(1)
  }

  "Inputvaluevalidations" should "fire if an ID is too long" in {
    val todoAlias = server
      .query(
        """mutation {
          |  createOneTodo(
          |    data: {
          |      title: "new title1"
          |      alias: "todo1"
          |    }
          |  ) {
          |    alias
          |  }
          |}
        """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.createOneTodo.alias")

    todoCount should be(1)

    val result = server.query(
      s"""mutation {
         |  upsertOneTodo(
         |    where: {alias: "$todoAlias"}
         |    create: {
         |      title: "irrelevant"
         |      alias: "irrelevant"
         |    }
         |    update: {
         |      title: { set: "updated title" }
         |    }
         |  ){
         |    id
         |    title
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should be("updated title")

    todoCount should be(1)
  }

  "An upsert" should "perform only an update if the update changes the unique field used in the where clause" in {
    val todoId = server
      .query(
        """mutation {
          |  createOneTodo(
          |    data: {
          |      title: "title"
          |      alias: "todo1"
          |    }
          |  ) {
          |    id
          |  }
          |}
        """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.createOneTodo.id")

    todoCount should be(1)

    val result = server.query(
      s"""mutation {
         |  upsertOneTodo(
         |    where: {alias: "todo1"}
         |    create: {
         |      title: "title of new node"
         |      alias: "alias-of-new-node"
         |    }
         |    update: {
         |      title: { set: "updated title" }
         |      alias: { set:"todo1-new" }
         |    }
         |  ){
         |    id
         |    title
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should equal("updated title")
    todoCount should be(1)

    // the original node has been updated
    server
      .query(
        s"""{
        |  findOneTodo(where: {id: "$todoId"}){
        |    title
        |  }
        |}
      """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.findOneTodo.title") should equal("updated title")
  }

  "An upsert" should "perform only an update if the update changes nothing" in {
    val todoId = server
      .query(
        """mutation {
          |  createOneTodo(
          |    data: {
          |      title: "title"
          |      alias: "todo1"
          |    }
          |  ) {
          |    id
          |  }
          |}
        """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.createOneTodo.id")

    todoCount should be(1)

    val result = server.query(
      s"""mutation {
         |  upsertOneTodo(
         |    where: {alias: "todo1"}
         |    create: {
         |      title: "title of new node"
         |      alias: "alias-of-new-node"
         |    }
         |    update: {
         |      title: { set: "title" }
         |      alias: { set: "todo1" }
         |    }
         |  ){
         |    id
         |    title
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsString("data.upsertOneTodo.title") should equal("title")
    todoCount should be(1)
    // the original node has been updated
    server
      .query(
        s"""{
           |  findOneTodo(where: {id: "$todoId"}){
           |    title
           |  }
           |}
      """.stripMargin,
        project,
        legacy = false,
      )
      .pathAsString("data.findOneTodo.title") should equal("title")
  }

  "An upsertOne mutation" should "correctly apply all number operations for Int on update" in {
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

  "An upsertOne mutation" should "correctly apply all number operations for Float on update" in {
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

  def queryNumberOperation(project: Project, id: Int, field: String, op: String, value: String): String = {
    val result = server.query(
      s"""mutation {
         |  upsertOneTestModel(
         |    where: { id: $id }
         |    create: { id: $id }
         |    update: { $field: { $op: $value } }
         |  ){
         |    $field
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.upsertOneTestModel").toString
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

  def todoCount: Int = {
    val result = server.query(
      "{ findManyTodo { id } }",
      project,
      legacy = false,
    )
    result.pathAsSeq("data.findManyTodo").size
  }
}
