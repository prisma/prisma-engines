package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class SubqueryTooManyColumnsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "Subquery has too many columns " should "not occur" in {

    val project = ProjectDsl.fromString {
      s"""
       |model User {
       |  id         Int     @id
       |  name       String?
       |  friends    User[]  @relation("UserfriendOf")
       |  friendOf   User?   @relation("UserfriendOf", fields: [friendOfId], references: [id])
       |  friendOfId Int?
       |}
       """.stripMargin
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" friendOf:{ create:{ name: "B", id: 2}}}){
        |    id
        |    friends { id }
        |    friendOf{ id }
        |  }
        |}
      """.stripMargin,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"friends\":[],\"friendOf\":{\"id\":2}}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 3, name: "C" friendOf:{ create:{ name: "D", id: 4}}}){
        |    id
        |    friends { id }
        |    friendOf{ id }
        |  }
        |}
      """.stripMargin,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":3,\"friends\":[],\"friendOf\":{\"id\":4}}}}")

    val result = server.query(
      """{users(where: { friendOf:{ is:{ name: {contains: "B"}}}}){
      |    id
      |    friends { id, name}
      |    friendOf{ id, name }
      |  }
      |}
      """.stripMargin,
      project
    )

    result.toString() should be("{\"data\":{\"users\":[]}}")
  }

}
