package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class SubqueryTooManyColumnsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "A relation filter on a 1:M self relation " should "work" in {

    for (fieldName <- Vector("field_a", "field_z")) {
      val project = ProjectDsl.fromString {
        s"""
           |model User {
           |  id         Int     @id
           |  name       String?
           |  field_b    User[]  @relation("UserfriendOf")
           |  $fieldName   User?    @relation("UserfriendOf", fields: [${fieldName}Id], references: [id])
           |  ${fieldName}Id Int?
           |}
       """
      }
      database.setup(project)

      val setup = server.query(
        s"""mutation{createUser(data: { id: 1, name: "A" $fieldName:{ create:{ id: 10, name: "AA"}}}){
          |    id
          |    field_b { id }
          |    $fieldName{ id }
          |  }
          |}
      """,
        project
      )

      val setup_res = s"""{\"data\":{\"createUser\":{\"id\":1,\"field_b\":[],\"$fieldName\":{\"id\":10}}}}"""

      setup.toString() should be(setup_res)

      val setup2 = server.query(
        s"""mutation{createUser(data: { id: 2, name: "B" $fieldName:{ create:{ id: 20, name: "BB"}}}){
          |    id
          |    field_b { id }
          |    $fieldName{ id }
          |  }
          |}
      """,
        project
      )

      val setup2_res = s"""{\"data\":{\"createUser\":{\"id\":2,\"field_b\":[],\"$fieldName\":{\"id\":20}}}}"""

      setup2.toString() should be(setup2_res)

      val result = server.query(
        s"""{users(where: { $fieldName:{ is:{ name: {contains: "B"}}}}){
          |    id
          |    field_b { id, name}
          |    $fieldName{ id, name }
          |  }
          |}
      """,
        project
      )

      val result_res = s"""{\"data\":{\"users\":[{\"id\":2,\"field_b\":[],\"$fieldName\":{\"id\":20,\"name\":\"BB\"}}]}}"""
      result.toString() should be(result_res)

      val result2 = server.query(
        s"""{users(where: { field_b:{ some:{ name: {contains: "B"}}}}){
          |    id
          |    field_b { id, name}
          |    $fieldName{ id, name }
          |  }
          |}
      """,
        project
      )

      val result2_res = s"""{\"data\":{\"users\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"$fieldName\":null}]}}"""
      result2.toString() should be(result2_res)
    }
  }

  "A relation filter on a N:M self relation " should "work" in {

    for (fieldName <- Vector("field_a", "field_z")) {
      val project = ProjectDsl.fromString {
        s"""
         |model User {
         |  id         Int     @id
         |  name       String?
         |  field_b    User[]  @relation("UserfriendOf")
         |  $fieldName    User[] @relation("UserfriendOf")
         |  ${fieldName}Id  Int?
         |}
       """
      }
      database.setup(project)

      val setup = server.query(
        s"""mutation{createUser(data: { id: 1, name: "A" $fieldName:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_b { id }
        |    $fieldName{ id }
        |  }
        |}
      """,
        project
      )

      setup.toString() should be(s"""{\"data\":{\"createUser\":{\"id\":1,\"field_b\":[],\"$fieldName\":[{\"id\":10}]}}}""")

      val setup2 = server.query(
        s"""mutation{createUser(data: { id: 2, name: "B" $fieldName:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_b { id }
        |    $fieldName{ id }
        |  }
        |}
      """,
        project
      )

      setup2.toString() should be(s"""{\"data\":{\"createUser\":{\"id\":2,\"field_b\":[],\"$fieldName\":[{\"id\":20}]}}}""")

      val result = server.query(
        s"""{users(where: { $fieldName:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    $fieldName{ id, name }
        |  }
        |}
      """,
        project
      )

      result.toString() should be(s"""{\"data\":{\"users\":[{\"id\":2,\"field_b\":[],\"$fieldName\":[{\"id\":20,\"name\":\"BB\"}]}]}}""")

      val result2 = server.query(
        s"""{users(where: { field_b:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    $fieldName{ id, name }
        |  }
        |}
      """,
        project
      )

      result2.toString() should be(s"""{\"data\":{\"users\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"$fieldName\":[]}]}}""")
    }
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
