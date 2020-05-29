package queries.relations

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class InlineRelationSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Querying the scalar field that backs a relation and the relation itself" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model ModelA {
         |  id   String  @id
         |  bool Boolean @default(true)
         |  b_id String?
         |  b    ModelB? @relation(fields: [b_id], references: [id])
         |}
         |
         |model ModelB {
         |  id  String @id
         |}
       """.stripMargin
    }
    database.setup(project)

    server.query(
      """
        |mutation {
        |  createOneModelA(data: { id: "1" }){
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    val result = server.query(
      """
        |{
        |  findManyModelA {
        |    id
        |    b_id
        |    b {
        |      id
        |    }
        |    bool
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyModelA":[{"id":"1","b_id":null,"b":null,"bool":true}]}}""")
  }
}
