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
       |  field_a    User[]  @relation("UserfriendOf")
       |  field_b   User?    @relation("UserfriendOf", fields: [field_bId], references: [id])
       |  field_bId Int?
       |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" field_b:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_a { id }
        |    field_b{ id }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"field_a\":[],\"field_b\":{\"id\":10}}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 2, name: "B" field_b:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_a { id }
        |    field_b{ id }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":2,\"field_a\":[],\"field_b\":{\"id\":20}}}}")

    val result = server.query(
      """{users(where: { field_b:{ is:{ name: {contains: "B"}}}}){
      |    id
      |    field_a { id, name}
      |    field_b{ id, name }
      |  }
      |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"users\":[{\"id\":2,\"field_a\":[],\"field_b\":{\"id\":20,\"name\":\"BB\"}}]}}")

    val result2 = server.query(
      """{users(where: { field_a:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_a { id, name}
        |    field_b{ id, name }
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id\":20,\"field_a\":[{\"id\":2,\"name\":\"B\"}],\"field_b\":null}]}}")

  }

  "Subquery has too many columns " should "not occur 2" in {

    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id         Int     @id
         |  name       String?
         |  posts      Post[]  @relation("UserPost")
         |}
         |
         |model Post {
         |  id         Int     @id
         |  name       String?
         |  user       User?   @relation("UserPost", fields: [userId], references: [id])
         |  userId Int?
         |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" posts:{ create:{ name: "AA", id: 10}}}){
        |    id
        |    posts { id, name }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"posts\":[{\"id\":10,\"name\":\"AA\"}]}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 2, name: "B" posts:{ create:{ name: "BB", id: 20}}}){
        |    id
        |    posts { id, name }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":2,\"posts\":[{\"id\":20,\"name\":\"BB\"}]}}}")

    val result = server.query(
      """{posts(where: { user:{ is:{ name: {contains: "B"}}}}){
        |    id
        |    name
        |    user { id, name}
        |  }
        |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"posts\":[{\"id\":20,\"name\":\"BB\",\"user\":{\"id\":2,\"name\":\"B\"}}]}}")

    val result2 = server.query(
      """{users(where: { posts:{ some:{ name: {contains: "BB"}}}}){
        |    id
        |    name
        |    posts { id, name}
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id\":2,\"name\":\"B\",\"posts\":[{\"id\":20,\"name\":\"BB\"}]}]}}")
  }

}
