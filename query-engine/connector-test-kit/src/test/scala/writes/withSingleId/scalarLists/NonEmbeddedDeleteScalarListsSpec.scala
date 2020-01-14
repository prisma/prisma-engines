package writes.withSingleId.scalarLists

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.{JoinRelationLinksCapability, ScalarListsCapability}
import util._

class NonEmbeddedDeleteScalarListsSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(ScalarListsCapability, JoinRelationLinksCapability)

  "A nested delete  mutation" should "also delete ListTable entries" in {

    val project: Project = SchemaDsl.fromStringV11() {
      s"""model Top {
        | id     String  @id @default(cuid())
        | name   String  @unique
        | bottom Bottom? @relation(references: [id])
        |}
        |
        |model Bottom {
        | id   String @id @default(cuid())
        | name String @unique
        | list Int[]
        |}"""
    }

    database.setup(project)

    server.query(
      """mutation {
        |  createTop(
        |    data: { name: "test", bottom: {create: {name: "test2", list: {set: [1,2,3]}} }}
        |  ){
        |    name
        |    bottom{name, list}
        |  }
        |}
      """,
      project
    )

    val res = server.query("""mutation{updateTop(where:{name:"test" }data: {bottom: {delete: true}}){name, bottom{name}}}""", project)

    res.toString should be("""{"data":{"updateTop":{"name":"test","bottom":null}}}""")
  }

  "A cascading delete  mutation" should "also delete ListTable entries" ignore  { // TODO: Remove ignore when cascading again

    val project: Project = SchemaDsl.fromStringV11() {
      s"""model Top {
        |  id     String  @id @default(cuid())
        |  name   String  @unique
        |  bottom Bottom? @relation(name: "Test", onDelete: CASCADE, references: [id])
        |}
        |
        |model Bottom {
        |  id   String @id @default(cuid())
        |  name String @unique
        |  list Int[]
        |  top  Top    @relation(name: "Test")
        |}"""
    }

    database.setup(project)

    server.query(
      """mutation {
        |  createTop(
        |    data: { name: "test", bottom: {create: {name: "test2", list: {set: [1,2,3]}} }}
        |  ){
        |    name
        |    bottom{name, list}
        |  }
        |}
      """,
      project
    )

    server.query("""mutation{deleteTop(where:{name:"test"}){name}}""", project)

    server.query("""query{tops{name}}""", project).toString() should be("""{"data":{"tops":[]}}""")
    server.query("""query{bottoms{name}}""", project).toString() should be("""{"data":{"bottoms":[]}}""")
  }
}
