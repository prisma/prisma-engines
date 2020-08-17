package queries.filters

import org.scalatest._
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class RelationIsNullSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val project = SchemaDsl.fromStringV11() {
    """
      |model Message {
      |  id          String  @id @default(cuid())
      |  messageName String?
      |  image_id    String?
      |
      |  image Image? @relation(fields: [image_id], references: [id])
      |}
      |
      |model Image {
      |  id        String   @id @default(cuid())
      |  imageName String?
      |
      |  message Message?
      |}
    """
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach() = {
    super.beforeEach()
    database.truncateProjectTables(project)

    // add data
    server.query(
      """mutation {createMessage(
        |     data: {
        |       messageName: "message 1",
        |       }
        |){messageName}}""",
      project = project
    )

    server.query(
      """mutation {createMessage(
        |     data: {
        |       messageName: "message 2",
        |       image:{create:{imageName:"image 1"}}
        |       }
        |){messageName}}""",
      project = project
    )

    server.query(
      """mutation {createImage(
        |     data: {
        |       imageName: "image 2"
        |       }
        |){imageName}}""",
      project = project
    )

  }

  "Filtering on whether a relation is null" should "work" in {
    server
      .query(
        query = """query {
                  |  images(where: { message: { is: null }}) {
                  |    imageName
                  |  }
                  |}""",
        project = project
      )
      .toString should be("""{"data":{"images":[{"imageName":"image 2"}]}}""")
  }

  "Filtering on whether a relation is null" should "work 2" in {
    server
      .query(
        query = """query {
                  |  messages(where: { image: { is: null }}) {
                  |    messageName
                  |  }
                  |}""",
        project = project
      )
      .toString should be("""{"data":{"messages":[{"messageName":"message 1"}]}}""")
  }
}
