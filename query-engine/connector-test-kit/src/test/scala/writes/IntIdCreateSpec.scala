package writes

import org.scalatest.{FlatSpec, Matchers}
import util._

class IntIdCreateSpec extends FlatSpec with Matchers with ApiSpecBase {


  "Creating an item with an id field of type Int without default" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createTodo(data: { title: "the title", id: 10 }){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsString("data.createTodo.title") should equal("the title")
    result.pathAsLong("data.createTodo.id") should equal(10)
  }

  "Creating an item with an id field of type Int without default without providing the id" should "error" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    server.queryThatMustFail(
      """
        |mutation {
        |  createTodo(data: { title: "the title" }){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project,0
    )
  }

  "Creating an item with an id field of type Int with static default" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id @default(0)
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createTodo(data: { title: "the title", id: 10 }){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsString("data.createTodo.title") should equal("the title")
    result.pathAsLong("data.createTodo.id") should equal(10)

    val result2 = server.query(
      """
        |mutation {
        |  createTodo(data: { title: "the title"}){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result2.pathAsString("data.createTodo.title") should equal("the title")
    result2.pathAsLong("data.createTodo.id") should equal(0)
  }

  "Creating an item with an id field of type Int with autoincrement" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id @default(autoincrement())
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createTodo(data: { title: "the title"}){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project
    )

    result.pathAsString("data.createTodo.title") should equal("the title")
    result.pathAsLong("data.createTodo.id") should equal(1)
  }

  "Creating an item with an id field of type Int with autoincrement and providing an id" should "error" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Todo {
         |  id    Int @id @default(autoincrement())
         |  title String
         |}
       """.stripMargin
    }
    database.setup(project)

    server.queryThatMustFail(
      """
        |mutation {
        |  createTodo(data: { title: "the title", id: 2}){
        |    id
        |    title
        |  }
        |}
      """.stripMargin,
      project,
      0
    )
  }

}
