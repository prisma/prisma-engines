package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class SubqueryTooManyColumnsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "A relation filter on a 1:M self relation " should "work" in {

    val project = ProjectDsl.fromString {
      s"""
       |model User {
       |  id         Int     @id
       |  name       String?
       |  field_b    User[]  @relation("UserfriendOf")
       |  field_a   User?    @relation("UserfriendOf", fields: [field_aId], references: [id])
       |  field_aId Int?
       |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" field_a:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_b { id }
        |    field_a{ id }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"field_b\":[],\"field_a\":{\"id\":10}}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 2, name: "B" field_a:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_b { id }
        |    field_a{ id }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":2,\"field_b\":[],\"field_a\":{\"id\":20}}}}")

    val result = server.query(
      """{users(where: { field_a:{ is:{ name: {contains: "B"}}}}){
      |    id
      |    field_b { id, name}
      |    field_a{ id, name }
      |  }
      |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"users\":[{\"id\":2,\"field_b\":[],\"field_a\":{\"id\":20,\"name\":\"BB\"}}]}}")

    val result2 = server.query(
      """{users(where: { field_b:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_a{ id, name }
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"field_a\":null}]}}")

  }

  "A relation filter on a N:M self relation " should "work" in {

    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id         Int     @id
         |  name       String?
         |  field_b    User[]  @relation("UserfriendOf")
         |  field_a    User[] @relation("UserfriendOf")
         |  field_aId  Int?
         |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" field_a:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_b { id }
        |    field_a{ id }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"field_b\":[],\"field_a\":[{\"id\":10}]}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 2, name: "B" field_a:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_b { id }
        |    field_a{ id }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":2,\"field_b\":[],\"field_a\":[{\"id\":20}]}}}")

    val result = server.query(
      """{users(where: { field_a:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_a{ id, name }
        |  }
        |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"users\":[{\"id\":2,\"field_b\":[],\"field_a\":[{\"id\":20,\"name\":\"BB\"}]}]}}")

    val result2 = server.query(
      """{users(where: { field_b:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_a{ id, name }
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"field_a\":[]}]}}")

  }

  "A relation filter on a 1:M self relation " should "work  with inverted lexicographic field order" in {

    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id         Int     @id
         |  name       String?
         |  field_b    User[]  @relation("UserfriendOf")
         |  field_z   User?    @relation("UserfriendOf", fields: [field_zId], references: [id])
         |  field_zId Int?
         |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" field_z:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_b { id }
        |    field_z{ id }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"field_b\":[],\"field_z\":{\"id\":10}}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 2, name: "B" field_z:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_b { id }
        |    field_z{ id }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":2,\"field_b\":[],\"field_z\":{\"id\":20}}}}")

    val result = server.query(
      """{users(where: { field_z:{ is:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_z{ id, name }
        |  }
        |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"users\":[{\"id\":2,\"field_b\":[],\"field_z\":{\"id\":20,\"name\":\"BB\"}}]}}")

    val result2 = server.query(
      """{users(where: { field_b:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_z{ id, name }
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"field_z\":null}]}}")

  }

  "A relation filter on a N:M self relation " should "work  with inverted lexicographic field order" in {

    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id         Int     @id
         |  name       String?
         |  field_b    User[]  @relation("UserfriendOf")
         |  field_z    User[] @relation("UserfriendOf")
         |  field_zId  Int?
         |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id: 1, name: "A" field_z:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_b { id }
        |    field_z{ id }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id\":1,\"field_b\":[],\"field_z\":[{\"id\":10}]}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id: 2, name: "B" field_z:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_b { id }
        |    field_z{ id }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id\":2,\"field_b\":[],\"field_z\":[{\"id\":20}]}}}")

    val result = server.query(
      """{users(where: { field_z:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_z{ id, name }
        |  }
        |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"users\":[{\"id\":2,\"field_b\":[],\"field_z\":[{\"id\":20,\"name\":\"BB\"}]}]}}")

    val result2 = server.query(
      """{users(where: { field_b:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    field_z{ id, name }
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"field_z\":[]}]}}")

  }

  "A relationfilter on a non-self relation" should "work" in {

    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id_user         Int     @id
         |  name       String?
         |  posts      Post[]  @relation("UserPost")
         |}
         |
         |model Post {
         |  id         Int     @id
         |  name       String?
         |  user       User?   @relation("UserPost", fields: [userId], references: [id_user])
         |  userId Int?
         |}
       """
    }
    database.setup(project)

    val setup = server.query(
      """mutation{createUser(data: { id_user: 1, name: "A" posts:{ create:{ name: "AA", id: 10}}}){
        |    id_user
        |    posts { id, name }
        |  }
        |}
      """,
      project
    )

    setup.toString() should be("{\"data\":{\"createUser\":{\"id_user\":1,\"posts\":[{\"id\":10,\"name\":\"AA\"}]}}}")

    val setup2 = server.query(
      """mutation{createUser(data: { id_user: 2, name: "B" posts:{ create:{ name: "BB", id: 20}}}){
        |    id_user
        |    posts { id, name }
        |  }
        |}
      """,
      project
    )

    setup2.toString() should be("{\"data\":{\"createUser\":{\"id_user\":2,\"posts\":[{\"id\":20,\"name\":\"BB\"}]}}}")

    val result = server.query(
      """{posts(where: { user:{ is:{ name: {contains: "B"}}}}){
        |    id
        |    name
        |    user { id_user, name}
        |  }
        |}
      """,
      project
    )

    result.toString() should be("{\"data\":{\"posts\":[{\"id\":20,\"name\":\"BB\",\"user\":{\"id_user\":2,\"name\":\"B\"}}]}}")

    val result2 = server.query(
      """{users(where: { posts:{ some:{ name: {contains: "BB"}}}}){
        |    id_user
        |    name
        |    posts { id, name}
        |  }
        |}
      """,
      project
    )

    result2.toString() should be("{\"data\":{\"users\":[{\"id_user\":2,\"name\":\"B\",\"posts\":[{\"id\":20,\"name\":\"BB\"}]}]}}")
  }

}

//Fixme M2M Selfrelation
