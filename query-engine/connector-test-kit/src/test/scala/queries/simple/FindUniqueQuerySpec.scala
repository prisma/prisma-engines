package queries.simple

import org.scalatest.{FlatSpec, Matchers}
import util._

class FindUniqueQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  "Fetching a unique record" should "work by id" in {
    val project = SchemaDsl.fromStringV11() {
      """model TestModel {
        |  id    Int    @id
        |}
      """.stripMargin
    }

    database.setup(project)

    server.query(
      """mutation {
        |  createOneTestModel(data: { id: 1 }) {
        |    id
        |  }
        |}""".stripMargin,
      project,
      legacy = false
    )

    val result = server.query(
      """{
        |  findUniqueTestModel(where: { id: 1 }) {
        |    id
        |  }
        |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString should be("""{"data":{"findUniqueTestModel":{"id":1}}}""")
  }

  "Fetching a unique record" should "work by any unique field" in {
    val project = SchemaDsl.fromStringV11() {
      """model TestModel {
        |  id   Int    @id
        |  uniq String @unique
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """mutation {
         |  createOneTestModel(data: {id: 1, uniq: "uniq"}) {
         |    id
         |  }
         |}""".stripMargin,
      project,
      legacy = false,
    )

    val result = server.query(
      """{
          |  findUniqueTestModel(where: { uniq: "uniq" }) {
          |    id
          |    uniq
          |  }
          |}""".stripMargin,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"findUniqueTestModel":{"id":1,"uniq":"uniq"}}}""")
  }
}
