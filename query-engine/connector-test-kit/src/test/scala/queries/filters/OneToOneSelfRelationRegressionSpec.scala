package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class OneToOneSelfRelationRegressionSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Querying a 1:1 self relation with nulls" should "work on both sides" in {
    val project = ProjectDsl.fromString {
      s"""
         |model User {
         |  id       Int     @id
         |  name     String?
         |  friendOf User?   @relation("Userfriend")
         |  friend   User?   @relation("Userfriend", fields: [friendId], references: [id])
         |  friendId Int?
         |}
       """.stripMargin
    }
    database.setup(project)

    val create_1 = server.query(
      """
        |mutation {
        |  createUser(data: { id: 1, name: "Bob"}){
        |    id
        |    name
        |    friend {name}
        |    friendOf{name}
        |
        |  }
        |}
      """,
      project
    )
    create_1.toString should be("{\"data\":{\"createUser\":{\"id\":1,\"name\":\"Bob\",\"friend\":null,\"friendOf\":null}}}")

    val create_2 = server.query(
      """
        |mutation {
        |  createUser(data: { id: 2, name: "Alice", friend: {connect:{id: 1}}}){
        |    id
        |    name
        |    friend {name}
        |    friendOf{name}
        |  }
        |}
      """,
      project
    )

    create_2.toString should be("{\"data\":{\"createUser\":{\"id\":2,\"name\":\"Alice\",\"friend\":{\"name\":\"Bob\"},\"friendOf\":null}}}")

    val find_1 = server.query(
      """
        |query {
        |  users(where: {friend: null }){
        |    id
        |    name
        |    friend {name}
        |    friendOf{name}
        |  }
        |}
      """,
      project
    )

    find_1.toString should be("{\"data\":{\"users\":[{\"id\":1,\"name\":\"Bob\",\"friend\":null,\"friendOf\":{\"name\":\"Alice\"}}]}}")

    val find_2 = server.query(
      """
        query {
        |  users(where: {friendOf: null }){
        |    id
        |    name
        |    friend {name}
        |    friendOf{name}
        |  }
        |}
      """,
      project
    )

    find_2.toString should be("{\"data\":{\"users\":[{\"id\":2,\"name\":\"Alice\",\"friend\":{\"name\":\"Bob\"},\"friendOf\":null}]}}")

  }
}
