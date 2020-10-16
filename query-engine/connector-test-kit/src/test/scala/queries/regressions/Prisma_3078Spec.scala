package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class Prisma_3078Spec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  // validates fix for
  //https://github.com/prisma/prisma/issues/3078
  //https://github.com/prisma/prisma-client-js/issues/550

  // The relationfilter logic for Selfrelations was sensitive to the side from which the filter traversed as well as the
  // naming of the relationfields since this fed into the RelationSide logic. This tests traversal from both sides as well
  // as switching the lexicographic order of the relation fields.

  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "A relation filter on a 1:1 self relation " should "work" taggedAs (IgnoreMsSql) in {

    for (fieldName <- Vector("field_a", "field_z")) {
      val project = ProjectDsl.fromString {
        s"""
           |model User {
           |  id              Int       @id
           |  name            String?
           |  field_b         User?      @relation("UserfriendOf")
           |  $fieldName      User?     @relation("UserfriendOf", fields: [${fieldName}Id], references: [id])
           |  ${fieldName}Id  Int?
           |}
       """
      }
      database.setup(project)

      val setup = server.query(
        s"""mutation{createOneUser(data: { id: 1, name: "A" $fieldName:{ create:{ id: 10, name: "AA"}}}){
           |    id
           |    field_b { id }
           |    $fieldName{ id }
           |  }
           |}
      """,
        project,
        legacy = false
      )

      val setup_res = s"""{\"data\":{\"createOneUser\":{\"id\":1,\"field_b\":null,\"$fieldName\":{\"id\":10}}}}"""

      setup.toString() should be(setup_res)

      val setup2 = server.query(
        s"""mutation{createOneUser(data: { id: 2, name: "B" $fieldName:{ create:{ id: 20, name: "BB"}}}){
           |    id
           |    field_b { id }
           |    $fieldName{ id }
           |  }
           |}
      """,
        project,
        legacy = false
      )

      val setup2_res = s"""{\"data\":{\"createOneUser\":{\"id\":2,\"field_b\":null,\"$fieldName\":{\"id\":20}}}}"""

      setup2.toString() should be(setup2_res)

      val result = server.query(
        s"""{findManyUser(where: { $fieldName:{ is:{ name: {contains: "B"}}}}){
           |    id
           |    field_b { id, name}
           |    $fieldName{ id, name }
           |  }
           |}
      """,
        project,
        legacy = false
      )

      val result_res = s"""{\"data\":{\"findManyUser\":[{\"id\":2,\"field_b\":null,\"$fieldName\":{\"id\":20,\"name\":\"BB\"}}]}}"""
      result.toString() should be(result_res)

      val result2 = server.query(
        s"""{findManyUser(where: { field_b:{ is:{ name: {contains: "B"}}}}){
           |    id
           |    field_b { id, name}
           |    $fieldName{ id, name }
           |  }
           |}
      """,
        project,
        legacy = false
      )

      val result2_res = s"""{\"data\":{\"findManyUser\":[{\"id\":20,\"field_b\":{\"id\":2,\"name\":\"B\"},\"$fieldName\":null}]}}"""
      result2.toString() should be(result2_res)
    }
  }

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
        s"""mutation{createOneUser(data: { id: 1, name: "A" $fieldName:{ create:{ id: 10, name: "AA"}}}){
          |    id
          |    field_b { id }
          |    $fieldName{ id }
          |  }
          |}
      """,
        project,
        legacy = false
      )

      val setup_res = s"""{\"data\":{\"createOneUser\":{\"id\":1,\"field_b\":[],\"$fieldName\":{\"id\":10}}}}"""

      setup.toString() should be(setup_res)

      val setup2 = server.query(
        s"""mutation{createOneUser(data: { id: 2, name: "B" $fieldName:{ create:{ id: 20, name: "BB"}}}){
          |    id
          |    field_b { id }
          |    $fieldName{ id }
          |  }
          |}
      """,
        project,
        legacy = false
      )

      val setup2_res = s"""{\"data\":{\"createOneUser\":{\"id\":2,\"field_b\":[],\"$fieldName\":{\"id\":20}}}}"""

      setup2.toString() should be(setup2_res)

      val result = server.query(
        s"""{findManyUser(where: { $fieldName:{ is:{ name: {contains: "B"}}}}){
          |    id
          |    field_b { id, name}
          |    $fieldName{ id, name }
          |  }
          |}
      """,
        project,
        legacy = false
      )

      val result_res = s"""{\"data\":{\"findManyUser\":[{\"id\":2,\"field_b\":[],\"$fieldName\":{\"id\":20,\"name\":\"BB\"}}]}}"""
      result.toString() should be(result_res)

      val result2 = server.query(
        s"""{findManyUser(where: { field_b:{ some:{ name: {contains: "B"}}}}){
          |    id
          |    field_b { id, name}
          |    $fieldName{ id, name }
          |  }
          |}
      """,
        project,
        legacy = false
      )

      val result2_res = s"""{\"data\":{\"findManyUser\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"$fieldName\":null}]}}"""
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
        s"""mutation{createOneUser(data: { id: 1, name: "A" $fieldName:{ create:{ id: 10, name: "AA"}}}){
        |    id
        |    field_b { id }
        |    $fieldName{ id }
        |  }
        |}
      """,
        project,
        legacy = false
      )

      setup.toString() should be(s"""{\"data\":{\"createOneUser\":{\"id\":1,\"field_b\":[],\"$fieldName\":[{\"id\":10}]}}}""")

      val setup2 = server.query(
        s"""mutation{createOneUser(data: { id: 2, name: "B" $fieldName:{ create:{ id: 20, name: "BB"}}}){
        |    id
        |    field_b { id }
        |    $fieldName{ id }
        |  }
        |}
      """,
        project,
        legacy = false
      )

      setup2.toString() should be(s"""{\"data\":{\"createOneUser\":{\"id\":2,\"field_b\":[],\"$fieldName\":[{\"id\":20}]}}}""")

      val result = server.query(
        s"""{findManyUser(where: { $fieldName:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    $fieldName{ id, name }
        |  }
        |}
      """,
        project,
        legacy = false
      )

      result.toString() should be(s"""{\"data\":{\"findManyUser\":[{\"id\":2,\"field_b\":[],\"$fieldName\":[{\"id\":20,\"name\":\"BB\"}]}]}}""")

      val result2 = server.query(
        s"""{findManyUser(where: { field_b:{ some:{ name: {contains: "B"}}}}){
        |    id
        |    field_b { id, name}
        |    $fieldName{ id, name }
        |  }
        |}
      """,
        project,
        legacy = false
      )

      result2.toString() should be(s"""{\"data\":{\"findManyUser\":[{\"id\":20,\"field_b\":[{\"id\":2,\"name\":\"B\"}],\"$fieldName\":[]}]}}""")
    }
  }
}
