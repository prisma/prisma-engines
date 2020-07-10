package writes.ids

import org.scalatest.{FlatSpec, Matchers}
import util._

class IntIdUpdateSpec extends FlatSpec with Matchers with ApiSpecBase {

  "Updating an item with an id field of type Int without default" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    // Setup
    val res = server.query(
      s"""mutation {
         |  createTodo(data: {title: "initial", id: 12}) {title, id}
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createTodo":{"title":"initial","id":12}}}""")

    // Check
    val result = server.query(
      """
        |mutation {
        |  updateTodo(where: {id: 12}, data: {title: "the title"}){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsString("data.updateTodo.title") should equal("the title")
    result.pathAsLong("data.updateTodo.id") should equal(12)
  }

  "Updating an item with an id field of type Int with static default" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id @default(0)
         |  title String
         |}
       """.stripMargin
    }

    database.setup(project)

    // Setup
    val res = server.query(
      s"""mutation {
         |  createTodo(data: {title: "initial", id: 12}) {title, id}
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createTodo":{"title":"initial","id":12}}}""")

    // Check
    val result = server.query(
      """
        |mutation {
        |  updateTodo(where: {id: 12}, data: { title: "the title" }){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsString("data.updateTodo.title") should equal("the title")
    result.pathAsLong("data.updateTodo.id") should equal(12)
  }

  "Updating an item with an id field of type Int with autoincrement" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id @default(autoincrement())
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    // Setup
    val res = server.query(
      s"""mutation {
         |  createTodo(data: {title: "initial"}) {title, id}
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createTodo":{"title":"initial","id":1}}}""")

    // Check
    val result = server.query(
      """
        |mutation {
        |  updateTodo(where: {id: 1}, data: {title: "the title"}){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsString("data.updateTodo.title") should equal("the title")
    result.pathAsLong("data.updateTodo.id") should equal(1)
  }

  "Updating the id of an item with an id field of type Int with autoincrement" should "error" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id @default(autoincrement())
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    // Setup
    val res = server.query(
      s"""mutation {
         |  createTodo(data: {title: "initial"}) {title, id}
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createTodo":{"title":"initial","id":1}}}""")

    // Check
    server.queryThatMustFail(
      """
        |mutation {
        |  updateTodo(where: {id: 1}, data: { title: "the title", id: 2}){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      errorContains = "Failed to validate the query `Error occurred during query validation & transformation:\\nMutation (object)\\n  ↳ updateTodo (field)\\n    ↳ data (argument)\\n      ↳ TodoUpdateInput (object)\\n        ↳ id (field)\\n          ↳ Field does not exist on enclosing type.` at `.Mutation.updateTodo.data.TodoUpdateInput.id"
    )
  }

/*
  "Updating a field of type Int with autoincrement" should "error" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id         String @id
         |  counter    Int @default(autoincrement())
         |  title      String
         |}
       """.stripMargin
    }
    database.setup(project)

    // Setup
    val res = server.query(
      s"""mutation {
         |  createTodo(data: {id: "the-id", title: "initial"}) {title, id, counter}
         |}""",
      project = project
    )

    res.toString should be(s"""{"data":{"createTodo":{"title":"initial","id":"the-id","counter":1}}}""")

    // Check
    server.queryThatMustFail(
      """
        |mutation {
        |  updateTodo(where: {id: "the-id"}, data: { counter: 7}){
        |    id
        |    title
        |    counter
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      errorContains = "Failed to validate the query `Error occurred during query validation & transformation:\\nMutation (object)\\n  ↳ updateTodo (field)\\n    ↳ data (argument)\\n      ↳ TodoUpdateInput (object)\\n        ↳ id (field)\\n          ↳ Field does not exist on enclosing type.` at `.Mutation.updateTodo.data.TodoUpdateInput.id"
    )
  }
*/

}
