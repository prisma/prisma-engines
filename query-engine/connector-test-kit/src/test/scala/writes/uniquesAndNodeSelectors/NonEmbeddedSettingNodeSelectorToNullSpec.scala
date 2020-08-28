package writes.uniquesAndNodeSelectors

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NonEmbeddedSettingNodeSelectorToNullSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "Setting a where value to null " should "should only update one if there are several nulls for the specified node selector" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model A {
        |  id   String  @id @default(cuid())
        |  b    String? @unique
        |  key  String  @unique
        |  c_id String?
        |
        |  c C? @relation(fields: [c_id], references: [id])
        |}
        |
        |model C {
        |  id String  @id @default(cuid())
        |  c  String?
        |}
      """
    }
    database.setup(project)

    server.query(
      """
        |mutation a {
        |  createOneA(data: { b: "abc", key: "abc", c: { create: { c: "C" } } }) {
        |    id
        |    key
        |    b
        |    c {
        |      c
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    server.query(
      """
        |mutation a {
        |  createOneA(data: { b: null, key: "abc2", c: { create: { c: "C2" } } }) {
        |    key
        |    b
        |    c {
        |      c
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    server.query(
      """
        |mutation b {
        |  updateOneA(
        |    where: { b: "abc" }
        |    data: { b: { set: null }, c: { update: { c: { set: "NewC" } } } }
        |  ) {
        |    b
        |    c {
        |      c
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    val result = server.query(
      """
        |{
        |  findManyA(orderBy: { id: asc }) {
        |    b
        |    c {
        |      c
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyA":[{"b":null,"c":{"c":"NewC"}},{"b":null,"c":{"c":"C2"}}]}}""")
  }
}
