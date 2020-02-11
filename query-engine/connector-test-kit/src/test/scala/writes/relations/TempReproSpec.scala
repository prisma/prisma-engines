package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class TempReproSpec extends FlatSpec with Matchers with ApiSpecBase {

  val schema =
    """
      |model ParentRec {
      |  id        String     @default(cuid()) @id
      |  createdAt DateTime   @default(now()) @map(name: "created_at")
      |  updatedAt DateTime   @updatedAt @map(name: "updated_at")
      |  code      String     @unique
      |  children  ChildRec[]
      |  @@map(name: "parent")
      |}
      |
      |model ChildRec {
      |  @@map(name: "child")
      |  id        String    @default(cuid()) @id
      |  createdAt DateTime  @default(now()) @map(name: "created_at")
      |  updatedAt DateTime  @updatedAt @map(name: "updated_at")
      |  code      String    @unique
      |  parent    ParentRec @map(name: "parent_id")
      |}
      |"""

  val project = SchemaDsl.fromString(schema)

  "updating a parent node" should "not crash" in {
    database.setup(project)

    server.query(
      s"""
         |mutation {
         |  createParentRec(data: {
         |    code: "parent"
         |    children: {
         |      create: [
         |        {
         |          code: "child"
         |        }
         |      ]
         |    }
         |  }) {
         |    id
         |    createdAt
         |    updatedAt
         |    code
         |    children {
         |      id
         |      createdAt
         |      updatedAt
         |      code
         |    }
         |  }
         |}
         |
         |""".stripMargin, project)


    server.query(
      s"""
         | mutation {
         |  updateParentRec(
         |    where: {
         |      code: "parent"
         |    }
         |    data: {
         |      children: {
         |        update: [
         |          {
         |            where: {
         |              code: "not_found"
         |            }
         |            data: {
         |              code: "child_upd"
         |            }
         |          }
         |        ]
         |      }
         |    }
         |  ) {
         |    id
         |    createdAt
         |    updatedAt
         |    code
         |    children {
         |      id
         |      createdAt
         |      updatedAt
         |      code
         |    }
         |  }
         |}
         |
         |""".stripMargin, project).toString should be("""{"data":{"todoes":[{"uTodo":"B"}]}}""")
  }
  }
