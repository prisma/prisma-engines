package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedAtomicNumberOperationsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  "An updateOne mutation with number operations on the top and updates on the child (inl. child)" should "handle id changes correctly" in {
    // Inline on the child
    val project = ProjectDsl.fromString {
      """model TestModel {
        |  id   Int           @id
        |  uniq Int           @unique
        |  rel  RelatedModel?
        |}
        |
        |model RelatedModel {
        | id    Int       @id
        | field String
        | tm_id Int
        | tm    TestModel @relation(fields: [tm_id], references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      s"""
         |mutation {
         |  createOneTestModel(
         |    data: {
         |      id: 1
         |      uniq: 2
         |      rel: { create: { id: 1, field: "field" } }
         |    }
         |  ) {
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    var result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { uniq: 2 }
         |    data: {
         |      id: { increment: 1 }
         |      uniq: { multiply: 3 }
         |      rel: {
         |        update: {
         |          field: { set: "updated" }
         |        }
         |      }
         |    }
         |  ){
         |    rel {
         |      id
         |    }
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel").toString should be("""{"rel":{"id":1}}""")

    result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { id: 2 }
         |    data: {
         |      id: { increment: 1 }
         |      uniq: { multiply: 3 }
         |      rel: {
         |        update: {
         |          field: { set: "updated 2" }
         |        }
         |      }
         |    }
         |  ){
         |    rel {
         |      id
         |      field
         |    }
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel").toString should be("""{"rel":{"id":1,"field":"updated 2"}}""")
  }

  "An updateOne mutation with number operations on the top and updates on the child (inl. parent)" should "handle id changes correctly" in {
    // Inline on the parent
    val project = ProjectDsl.fromString {
      """model TestModel {
        |  id     Int          @id
        |  uniq   Int          @unique
        |  rel_id Int
        |  rel    RelatedModel @relation(fields: [rel_id], references: [id])
        |}
        |
        |model RelatedModel {
        | id    Int       @id
        | field String
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      s"""
         |mutation {
         |  createOneTestModel(
         |    data: {
         |      id: 1
         |      uniq: 2
         |      rel: { create: { id: 1, field: "field" } }
         |    }
         |  ) {
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )

    var result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { uniq: 2 }
         |    data: {
         |      id: { increment: 1 }
         |      uniq: { multiply: 3 }
         |      rel: {
         |        update: {
         |          field: { set: "updated" }
         |        }
         |      }
         |    }
         |  ){
         |    rel {
         |      id
         |    }
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel").toString should be("""{"rel":{"id":1}}""")

    result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { id: 2 }
         |    data: {
         |      id: { increment: 1 }
         |      uniq: { multiply: 3 }
         |      rel: {
         |        update: {
         |          field: { set: "updated 2" }
         |        }
         |      }
         |    }
         |  ){
         |    rel {
         |      id
         |      field
         |    }
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel").toString should be("""{"rel":{"id":1,"field":"updated 2"}}""")
  }

  "A nested updateOne mutation" should "correctly apply all number operations for Int" in {
    val project = ProjectDsl.fromString {
      """model TestModel {
        |  id  Int           @id
        |  rel RelatedModel?
        |}
        |
        |model RelatedModel {
        | id       Int       @id
        | optInt   Int?
        | optFloat Float?
        | tm_id    Int
        | tm       TestModel @relation(fields: [tm_id], references: [id])
        |}
      """.stripMargin
    }

    database.setup(project)
    createTestModel(project, 1)
    createTestModel(project, 2, Some(3))

    // Increment
    queryNestedNumberOperation(project, 1, "optInt", "increment", "10") should be("""{"optInt":null}""")
    queryNestedNumberOperation(project, 2, "optInt", "increment", "10") should be("""{"optInt":13}""")

    // Decrement
    queryNestedNumberOperation(project, 1, "optInt", "decrement", "10") should be("""{"optInt":null}""")
    queryNestedNumberOperation(project, 2, "optInt", "decrement", "10") should be("""{"optInt":3}""")

    // Multiply
    queryNestedNumberOperation(project, 1, "optInt", "multiply", "2") should be("""{"optInt":null}""")
    queryNestedNumberOperation(project, 2, "optInt", "multiply", "2") should be("""{"optInt":6}""")

    // Divide
    queryNestedNumberOperation(project, 1, "optInt", "divide", "3") should be("""{"optInt":null}""")
    queryNestedNumberOperation(project, 2, "optInt", "divide", "3") should be("""{"optInt":2}""")

    // Set
    queryNestedNumberOperation(project, 1, "optInt", "set", "5") should be("""{"optInt":5}""")
    queryNestedNumberOperation(project, 2, "optInt", "set", "5") should be("""{"optInt":5}""")

    // Set null
    queryNestedNumberOperation(project, 1, "optInt", "set", "null") should be("""{"optInt":null}""")
    queryNestedNumberOperation(project, 2, "optInt", "set", "null") should be("""{"optInt":null}""")
  }

  "A nested updateOne mutation" should "correctly apply all number operations for Float" in {
    val project = ProjectDsl.fromString {
      """model TestModel {
        |  id  Int           @id
        |  rel RelatedModel?
        |}
        |
        |model RelatedModel {
        | id       Int    @id
        | optInt   Int?
        | optFloat Float?
        | tm_id    Int
        | tm       TestModel @relation(fields: [tm_id], references: [id])
        |}
      """.stripMargin
    }

    database.setup(project)
    createTestModel(project, 1)
    createTestModel(project, 2, None, Some(5.5))

    // Increment
    queryNestedNumberOperation(project, 1, "optFloat", "increment", "4.6") should be("""{"optFloat":null}""")
    queryNestedNumberOperation(project, 2, "optFloat", "increment", "4.6") should be("""{"optFloat":10.1}""")

    // Decrement
    queryNestedNumberOperation(project, 1, "optFloat", "decrement", "4.6") should be("""{"optFloat":null}""")
    queryNestedNumberOperation(project, 2, "optFloat", "decrement", "4.6") should be("""{"optFloat":5.5}""")

    // Multiply
    queryNestedNumberOperation(project, 1, "optFloat", "multiply", "2") should be("""{"optFloat":null}""")
    queryNestedNumberOperation(project, 2, "optFloat", "multiply", "2") should be("""{"optFloat":11}""")

    // Divide
    queryNestedNumberOperation(project, 1, "optFloat", "divide", "2") should be("""{"optFloat":null}""")
    queryNestedNumberOperation(project, 2, "optFloat", "divide", "2") should be("""{"optFloat":5.5}""")

    // Set
    queryNestedNumberOperation(project, 1, "optFloat", "set", "5.1") should be("""{"optFloat":5.1}""")
    queryNestedNumberOperation(project, 2, "optFloat", "set", "5.1") should be("""{"optFloat":5.1}""")

    // Set null
    queryNestedNumberOperation(project, 1, "optFloat", "set", "null") should be("""{"optFloat":null}""")
    queryNestedNumberOperation(project, 2, "optFloat", "set", "null") should be("""{"optFloat":null}""")
  }

  def queryNestedNumberOperation(project: Project, id: Int, field: String, op: String, value: String): String = {
    val result = server.query(
      s"""mutation {
         |  updateOneTestModel(
         |    where: { id: $id }
         |    data: { rel: { update: { $field: { $op: $value }}}}
         |  ){
         |    rel {
         |      $field
         |    }
         |  }
         |}
    """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.updateOneTestModel.rel").toString
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
         |      rel: {
         |        create: {
         |          id: $id
         |          optInt: $i
         |          optFloat: $f
         |        }
         |      }
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
