package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util._

class RelationDefaultsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  val schema =
    """model List{
      |   id     Int  @id @default(autoincrement())
      |   uList  String? @unique
      |   todoId Int? @default(1)
      |
      |   todo  Todo?   @relation(fields: [todoId], references: [id])
      |}
      |
      |model Todo{
      |   id    Int  @id @default(autoincrement())
      |   uTodo String? @unique
      |   lists  List[]
      |}"""

  val project = SchemaDsl.fromStringV11() { schema }

  "Not providing a value for an optional relation field with a default value" should "work" in {
    database.setup(project)

    server.query(s"""mutation {createList(data: {uList: "A", todo : { create: {uTodo: "B"}}}){id}}""", project)

    val result = server.query(s"""query{lists {uList, todo {uTodo}}}""", project)
    result.toString should equal("""{"data":{"lists":[{"uList":"A","todo":{"uTodo":"B"}}]}}""")

    server.query(s"""query{todoes {uTodo}}""", project).toString should be("""{"data":{"todoes":[{"uTodo":"B"}]}}""")

    countItems(project, "lists") should be(1)
    countItems(project, "todoes") should be(1)

    val result2 = server.query(s"""mutation { createList(data: {uList: "listWithTodoOne"}) { id todo { id } } }""", project)
    result2.toString should equal("""{"data":{"createList":{"id":2,"todo":{"id":1}}}}""")

    countItems(project, "lists") should be(2)

  }

  def countItems(project: Project, name: String): Int = {
    server.query(s"""query{$name{id}}""", project).pathAsSeq(s"data.$name").length
  }
}
