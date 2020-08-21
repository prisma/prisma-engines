package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NoErrorOnEmptyNestedDisconnectSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "A create followed by an update" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Report {
         |    id Int @id
         |    annotations Annotation[]
         |}
         |
         |model Annotation {
         |    id      Int     @id
         |    reports Report[]
         |}
       """.stripMargin
    }
    database.setup(project)

    val setup = server.query(
      """mutation {
        |  createReport(data: {
        |    id: 1
        |    annotations: {
        |      create: [{id: 1}]
        |    }
        |  }){
        |    id
        |    annotations{
        |       id
        |    }
        |  }
        |}""",
      project
    )

    setup.toString() should be("{\"data\":{\"createReport\":{\"id\":1,\"annotations\":[{\"id\":1}]}}}")

    val result = server.query(
      """mutation {
        |  updateReport(
        |  where: {  id: 1}
        |  data: {
        |    annotations: {
        |      disconnect: []
        |    }
        |  }){
        |    id
        |    annotations{
        |       id
        |    }
        |  }
        |}""",
      project
    )

    result.toString() should be("{\"data\":{\"updateReport\":{\"id\":1,\"annotations\":[{\"id\":1}]}}}")
  }
}
