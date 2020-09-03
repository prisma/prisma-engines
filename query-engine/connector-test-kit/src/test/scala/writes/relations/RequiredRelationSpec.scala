package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class RequiredRelationSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val schema =
    """
      | model List{
      |   id     Int  @id @default(autoincrement())
      |   name   String
      |   todoId Int
      |   todo   Todo   @relation(fields: [todoId], references: [id])
      | }
      |
      | model Todo{
      |   id     Int  @id @default(autoincrement())
      |   name   String
      |   lists  List[]
      | }
    """

  val project = SchemaDsl.fromStringV11() { schema }

  "Updating a required relation with null" should "return an error" in {
    database.setup(project)

    // Setup
    val result = server.query(
      """
        | mutation {
        |  createList(data: { name: "A", todo: { create: { name: "B" } } }) {
        |    id
        |    name
        |    todo {
        |      id
        |      name
        |    }
        |   }
        | }
      """,
      project
    )

    result.toString should equal("""{"data":{"createList":{"id":1,"name":"A","todo":{"id":1,"name":"B"}}}}""")

    // Check that the engine rejects `null` as a `TodoUpdateInput`.
    server.queryThatMustFail(
      """
        | mutation {
        |  updateList(where: { id: 1 }, data: { name: { set: "C" }, todo: null }) {
        |    name
        |    todo {
        |      id
        |      name
        |    }
        |   }
        | }
      """,
      project,
      errorCode = 2012,
      errorContains = "Missing a required value at `Mutation.updateList.data.ListUpdateInput.todo"
    )
  }

}
