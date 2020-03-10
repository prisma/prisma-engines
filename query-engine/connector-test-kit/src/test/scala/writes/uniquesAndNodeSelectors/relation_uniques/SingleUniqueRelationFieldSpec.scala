package writes.uniquesAndNodeSelectors.relation_uniques

import org.scalatest.{FlatSpec, Matchers}
import util._

// 1) Checks if relation fields in @unique in any constellation work with our mutations.
// Possible relation cardinalities:
// - 1!:1!
// - 1!:1
// - 1!:M
//
// 2) Checks basic cursor functionality.
class SingleUniqueRelationFieldSpec extends FlatSpec with Matchers with ApiSpecBase {
  // Mutations in this test:
  //  create         | root   | checked
  //  update         | root   | checked
  //  delete         | root   | not possible (1!:1!)
  //  upsert         | root   | checked
  //  updateMany     | root   | unnecessary
  //  deleteMany     | root   | unnecessary
  //  nested create  | create | checked
  //  nested update  | update | checked
  //  nested connect | -      | not possible (1!:1!)
  //  nested disconn | -      | not possible (1!:1!)
  //  nested delete  | -      | not possible (1!:1!)
  //  nested set     | -      | not possible (1!:1!)
  //  nested upsert  | -      | not possible (1!:1!)
  //  nested deleteM | -      | not possible (1!:1!)
  //  nested updateM | -      | not possible (1!:1!)
  "Using a unique that is also a 1!:1! single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  id    Int    @id
         |  child Child  @relation(references: [id]) @unique
         |  p     String
         |}
         |
         |model Child {
         |  id     Int    @id
         |  c      String
         |  parent Parent
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(
        |    data: {
        |      id: 1
        |      p: "Parent"
        |      child: {
        |        create: {
        |          id: 1,
        |          c: "Child"
        |        }
        |      }
        |    }
        |  ) {
        |    id
        |    p
        |    child{
        |       id
        |       c
        |    }
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"id\":1,\"p\":\"Parent\",\"child\":{\"id\":1,\"c\":\"Child\"}}}}")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(where: { child: 1 } data: { p: "UpdatedParent" }) {
        |    p
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("{\"data\":{\"updateParent\":{\"p\":\"UpdatedParent\"}}}")

    val res3 = server.query(
      """
        |mutation {
        |  updateChild(where: { id: 1 } data: { parent: { update: { p: "UpdatedFromChild" }}}) {
        |    parent { p }
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateChild\":{\"parent\":{\"p\":\"UpdatedFromChild\"}}}}")

    val res4 = server.query(
      """
        |mutation {
        |  upsertParent(
        |    where: { child: 2 }
        |    update: { p: "Doesn't matter" }
        |    create: { id: 2, p: "Parent2", child: { create: { id: 2, c: "Child2" } } }
        |  ) {
        |    id
        |    child {
        |      id
        |    }
        |  }
        |}
        |
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"id\":2,\"child\":{\"id\":2}}}}")
  }

  // Mutations in this test:
  //  create         | root   | checked
  //  update         | root   | checked
  //  delete         | root   | not possible (1!:1!)
  //  upsert         | root   | checked
  //  updateMany     | root   | unnecessary
  //  deleteMany     | root   | unnecessary
  //  nested create  | create | checked
  //  nested update  | update | checked
  //  nested connect | -      | not possible (1!:1!)
  //  nested disconn | -      | not possible (1!:1!)
  //  nested delete  | -      | not possible (1!:1!)
  //  nested set     | -      | not possible (1!:1!)
  //  nested upsert  | -      | not possible (1!:1!)
  //  nested deleteM | -      | not possible (1!:1!)
  //  nested updateM | -      | not possible (1!:1!)
  "Using a unique that is also a 1!:1! multi-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  id    Int    @id
         |  child Child  @relation(references: [id, c]) @unique
         |  p     String
         |}
         |
         |model Child {
         |  id     Int    @id
         |  c      String
         |  parent Parent
         |
         |  @@unique([id, c])
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(
        |    data: {
        |      id: 1
        |      p: "Parent"
        |      child: {
        |        create: {
        |          id: 1,
        |          c: "Child"
        |        }
        |      }
        |    }
        |  ) {
        |    id
        |    p
        |    child{
        |       id
        |       c
        |    }
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"id\":1,\"p\":\"Parent\",\"child\":{\"id\":1,\"c\":\"Child\"}}}}")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(where: { child: { child_id: 1, child_c: "Child" } } data: { p: "UpdatedParent" }) {
        |    p
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("{\"data\":{\"updateParent\":{\"p\":\"UpdatedParent\"}}}")

    val res3 = server.query(
      """
        |mutation {
        |  updateChild(where: { id: 1 } data: { parent: { update: { p: "UpdatedFromChild" }}}) {
        |    parent { p }
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateChild\":{\"parent\":{\"p\":\"UpdatedFromChild\"}}}}")

    // blocked by ME issue for now
//    val res4 = server.query(
//      """
//        |mutation {
//        |  upsertParent(
//        |    where:  { child: { child_id: 2, child_c: "Child2" } }
//        |    update: { p: "Doesn't matter" }
//        |    create: { id: 2, p: "Parent2", child: { create: { id: 2, c: "Child2" } } }
//        |  ) {
//        |    id
//        |    child {
//        |      id
//        |    }
//        |  }
//        |}
//        |
//      """,
//      project
//    )
//
//    res4.toString() should be("{\"data\":{\"upsertParent\":{\"id\":2,\"child\":{\"id\":2}}}}")
  }

  // WIP
}
