package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.ScalarListsCapability
import util._

class CreateMutationListSpec extends FlatSpec with Matchers with ApiSpecBase {

  override def runOnlyForCapabilities = Set(ScalarListsCapability)

  val schema =
    """
    |model User {
    |  id      Int      @id
    |  test    String[]
    |}
    |""".stripMargin

  val project = ProjectDsl.fromString { schema }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "A Create Mutation" should "should not accept null in set" in {
    server.queryThatMustFail(
      s"""mutation {createUser(data: {id: 1, test: {set: null}}){id, test }}""",
      project = project,
      errorCode = 2009
    )

    server
      .query(
        s"""mutation {  createUser(data: { id: 1}){id, test }}""",
        project = project
      )
      .toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"test\":[]}}}")
  }
}
