package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util._

class RelationDefaultsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  "Not providing a value for a required relation field with a default value" should "work" in {
    val schema =
      """
        | model List {
        |   id     Int  @id @default(autoincrement())
        |   name  String? @unique
        |   todoId Int @default(1)
        |
        |   todo  Todo   @relation(fields: [todoId], references: [id])
        | }
        |
        | model Todo{
        |   id    Int  @id @default(autoincrement())
        |   name String?
        |   lists  List[]
        | }
        """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Setup
    server.query("""
    | mutation {
    |   createList(data: { name: "A", todo: { create: { name: "B" } } }) {
    |     id
    |   }
    | }
    """,
                 project)

    val result = server.query("""
    | query {
    |   lists {
    |     name
    |     todo {
    |       name
    |     }
    |   }
    | }
    """,
                              project)
    result.toString should equal("""{"data":{"lists":[{"name":"A","todo":{"name":"B"}}]}}""")

    server.query(s"""query { todoes { name } }""", project).toString should be("""{"data":{"todoes":[{"name":"B"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    // Check that we can implicitly connect with the default value
    val result2 = server.query(
      """
    | mutation {
    |   createList(data: { name: "listWithTodoOne" }) {
    |     id
    |     todo {
    |       id
    |     }
    |   }
    | }
    """,
      project
    )
    result2.toString should equal("""{"data":{"createList":{"id":2,"todo":{"id":1}}}}""")

    countItems(project, "lists") should be(2)
  }

  // We ignore SQLite because a multi-column primary key cannot have an autoincrement column on SQLite.
  "Not providing a value for a required relation with multiple fields with one default value" should "not work" taggedAs IgnoreSQLite in {
    val schema =
      """
        | model List {
        |    id        Int  @id @default(autoincrement())
        |    name     String? @unique
        |    todoId    Int @default(1)
        |    todoName  String
        |    todo      Todo   @relation(fields: [todoId, todoName], references: [id, name])
        | }
        |
        | model Todo {
        |    id     Int @default(autoincrement())
        |    name  String
        |    lists  List[]
        |
        |    @@id([id, name])
        | }
      """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Setup
    server.query("""
      mutation {createList(data: {name: "A", todo : { create: {name: "B"}}}){id}}
    """,
                 project)

    val result = server.query(s"""query { lists { name, todo { name } } }""", project)
    result.toString should equal("""{"data":{"lists":[{"name":"A","todo":{"name":"B"}}]}}""")

    server.query("""query { todoes { name } }""", project).toString should be("""{"data":{"todoes":[{"name":"B"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    // Check that we still need to provide the name
    server.queryThatMustFail(
      s"""mutation { createList(data: { name: "listWithTodoOne" }) { id todo { id } } }""",
      project,
      errorCode = 2009,
      errorContains = "`Mutation.createList.data.ListCreateInput.todo`: A value is required but not set.",
    )

    countItems(project, "lists") should be(1)
  }

  // We ignore SQLite because a multi-column primary key cannot have an autoincrement column on SQLite.
  "Not providing a value for one field with a default in a required relation with multiple fields" should "work" taggedAs IgnoreSQLite in {
    val schema =
      """
        | model List {
        |    id        Int  @id @default(autoincrement())
        |    name     String? @unique
        |    todoId    Int @default(1)
        |    todoName  String
        |    todo      Todo   @relation(fields: [todoId, todoName], references: [id, name])
        | }
        |
        | model Todo {
        |    id     Int @default(autoincrement())
        |    name  String
        |    lists  List[]
        |
        |     @@id([id, name])
        | }
      """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Test that we can still create with the value without default only
    val result2 = server.query(
      """
      | mutation {
      |   createList(
      |     data: { name: "listWithTodoOne", todo: { create: { name: "abcd" } } }
      |   ) {
      |     id
      |     todo {
      |       id
      |     }
      |   }
      | }
    """,
      project
    )
    result2.toString should equal("""{"data":{"createList":{"id":1,"todo":{"id":1}}}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)
  }

  "Not providing a value for required relation fields with default values" should "work" in {
    val schema =
      """
        | model List {
        |   id       Int     @id @default(autoincrement())
        |   name    String? @unique
        |   todoId   Int     @default(1)
        |   todoName String  @default("theTodo")
        |   todo     Todo    @relation(fields: [todoId, todoName], references: [id, name])
        | }
        |
        | model Todo{
        |   id     Int     @default(1)
        |   name   String
        |   lists  List[]
        |   @@id([id, name])
        | }
        """

    val project = SchemaDsl.fromStringV11() { schema }

    database.setup(project)

    // Setup
    server.query(
      """
    | mutation {
    |   createList(data: { name: "A", todo: { create: { id: 1, name: "theTodo" } } }) {
    |     id
    |   }
    | }
    """,
      project
    )

    val result = server.query("""
    | query {
    |   lists {
    |     name
    |     todo {
    |       name
    |     }
    |   }
    | }
    """,
                              project)
    result.toString should equal("""{"data":{"lists":[{"name":"A","todo":{"name":"theTodo"}}]}}""")

    server.query(s"""query { todoes { name } }""", project).toString should be("""{"data":{"todoes":[{"name":"theTodo"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    // Check that we can implicitly connect with the default value
    val result2 = server.query(
      """
    | mutation {
    |   createList(data: { name: "listWithTheTodo" }) {
    |     id
    |     todo {
    |       id
    |       name
    |     }
    |   }
    | }
    """,
      project
    )
    result2.toString should equal("""{"data":{"createList":{"id":2,"todo":{"id":1,"name":"theTodo"}}}}""")

    countItems(project, "lists") should be(2)
  }

  def countItems(project: Project, name: String): Int = {
    server.query(s"""query{$name{id}}""", project).pathAsSeq(s"data.$name").length
  }
}
