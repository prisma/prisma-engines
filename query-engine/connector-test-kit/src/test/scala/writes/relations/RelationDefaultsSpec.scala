package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util._

class RelationDefaultsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  "Not providing a value for a required relation field with a default value" should "work" in {
    val schema =
      """
        | model List {
        |   id     Int  @id @default(autoincrement())
        |   uList  String? @unique
        |   todoId Int @default(1)
        |
        |   todo  Todo   @relation(fields: [todoId], references: [id])
        | }
        |
        | model Todo{
        |   id    Int  @id @default(autoincrement())
        |   uTodo String? @unique
        |   lists  List[]
        | }
        """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Setup
    server.query("""
    | mutation {
    |   createList(data: { uList: "A", todo: { create: { uTodo: "B" } } }) {
    |     id
    |   }
    | }
    """, project)

    val result = server.query("""
    | query {
    |   lists {
    |     uList
    |     todo {
    |       uTodo
    |     }
    |   }
    | }
    """, project)
    result.toString should equal("""{"data":{"lists":[{"uList":"A","todo":{"uTodo":"B"}}]}}""")

    server.query(s"""query { todoes { uTodo } }""", project).toString should be("""{"data":{"todoes":[{"uTodo":"B"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    // Check that we can implicitly connect with the default value
    val result2 = server.query("""
    | mutation {
    |   createList(data: { uList: "listWithTodoOne" }) {
    |     id
    |     todo {
    |       id
    |     }
    |   }
    | }
    """, project)
    result2.toString should equal("""{"data":{"createList":{"id":2,"todo":{"id":1}}}}""")

    countItems(project, "lists") should be(2)
  }

  // We ignore SQLite because a multi-column primary key cannot have an autoincrement column on SQLite.
  "Not providing a value for a required relation with multiple fields with one default value" should "not work" taggedAs IgnoreSQLite in {
    val schema =
      """
        | model List {
        |    id        Int  @id @default(autoincrement())
        |    uList     String? @unique
        |    todoId    Int @default(1)
        |    todoName  String
        |    todo      Todo   @relation(fields: [todoId, todoName], references: [id, uTodo])
        | }
        |
        | model Todo {
        |    id     Int @default(autoincrement())
        |    uTodo  String
        |    lists  List[]
        |
        |    @@id([id, uTodo])
        | }
      """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Setup
    server.query("""
      mutation {createList(data: {uList: "A", todo : { create: {uTodo: "B"}}}){id}}
    """, project)

    val result = server.query(s"""query { lists { uList, todo { uTodo } } }""", project)
    result.toString should equal("""{"data":{"lists":[{"uList":"A","todo":{"uTodo":"B"}}]}}""")

    server.query("""query { todoes { uTodo } }""", project).toString should be("""{"data":{"todoes":[{"uTodo":"B"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    // Check that we still need to provide the name
    server.queryThatMustFail(
      s"""mutation { createList(data: { uList: "listWithTodoOne" }) { id todo { id } } }""",
      project,
      errorCode = 2012,
      errorContains = "Missing a required value at `Mutation.createList.data.ListCreateInput.todo`",
    )

    countItems(project, "lists") should be(1)
  }

  // We ignore SQLite because a multi-column primary key cannot have an autoincrement column on SQLite.
  "Not providing a value for one field with a default in a required relation with multiple fields" should "work" taggedAs IgnoreSQLite in {
    val schema =
      """
        | model List {
        |    id        Int  @id @default(autoincrement())
        |    uList     String? @unique
        |    todoId    Int @default(1)
        |    todoName  String
        |    todo      Todo   @relation(fields: [todoId, todoName], references: [id, uTodo])
        | }
        |
        | model Todo {
        |    id     Int @default(autoincrement())
        |    uTodo  String
        |    lists  List[]
        |
        |     @@id([id, uTodo])
        | }
      """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Test that we can still create with the value without default only
    val result2 = server.query("""
      | mutation {
      |   createList(
      |     data: { uList: "listWithTodoOne", todo: { create: { uTodo: "abcd" } } }
      |   ) {
      |     id
      |     todo {
      |       id
      |     }
      |   }
      | }
    """, project)
    result2.toString should equal("""{"data":{"createList":{"id":1,"todo":{"id":1}}}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)
  }

    "Not providing a value for required relation fields with a default values" should "work" in {
    val schema =
      """
        | model List {
        |   id       Int     @id @default(autoincrement())
        |   uList    String? @unique
        |   todoId   Int     @default(1)
        |   todoName String  @default("theTodo")
        |   todo     Todo    @relation(fields: [todoId, todoName], references: [id, name])
        | }
        |
        | model Todo{
        |   id     Int     @default(1)
        |   name   String? @unique
        |   lists  List[]
        |   @@id([id, uTodo])
        | }
        """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Setup
    server.query("""
    | mutation {
    |   createList(data: { uList: "A", todo: { create: { id: 1, name: "theTodo" } } }) {
    |     id
    |   }
    | }
    """, project)

    val result = server.query("""
    | query {
    |   lists {
    |     uList
    |     todo {
    |       name
    |     }
    |   }
    | }
    """, project)
    result.toString should equal("""{"data":{"lists":[{"uList":"A","todo":{"name":"B"}}]}}""")

    server.query(s"""query { todoes { name } }""", project).toString should be("""{"data":{"todoes":[{"name":"theTodo"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    // Check that we can implicitly connect with the default value
    val result2 = server.query("""
    | mutation {
    |   createList(data: { uList: "listWithTheTodo" }) {
    |     id
    |     todo {
    |       id
    |       title
    |     }
    |   }
    | }
    """, project)
    result2.toString should equal("""{"data":{"createList":{"id":2,"todo":{"id":1,"title": "theTodo"}}}}""")

    countItems(project, "lists") should be(2)
  }


  def countItems(project: Project, name: String): Int = {
    server.query(s"""query{$name{id}}""", project).pathAsSeq(s"data.$name").length
  }
}
