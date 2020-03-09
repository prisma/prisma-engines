package writes.ids.relation_pks

import org.scalatest.{FlatSpec, Matchers}
import util._

// 1) Checks if relation fields in @@id in any constellation work with our mutations.
// Possible relation cardinalities:
// - 1!:1!
// - 1!:1
// - 1!:M
//
// 2) Checks basic cursor functionality.
class CompoundPKRelationFieldSpec extends FlatSpec with Matchers with ApiSpecBase {
  //todo @@Id
  // compound id contains simple relation + scalar
  // compound id contains compound relation field + scalar
  // compound id contains all compound relation fields + scalar
  // compound id is subset of compound relation field            (unlikely)

  //todo @@Unique
  // in place of @@id @@unique should behave similarly in most cases
  // exception: if the @@unique fields exactly match the database field(s) of the relation than the @(@)unique is dropped
  // the relation then becomes 1:1 in the datamodel
  // Problem: Table with one fk field that is marked unique
  // -> we generated 1:1 relation and don't put the unique on the datamodel
  // -> we then comment it out since the datamodel has no unique even though there is one on the db level
  // Solution: Either print the unique or treat 1:1 relation as a unique identifier

  // todo cursors
  // todo filters

  // Mutations in this test:
  //  create         | root   | checked
  //  update         | root   | checked
  //  delete         | root   | not possible
  //  upsert         | root   | checked
  //  updateMany     | root   | not possible
  //  deleteMany     | root   | not possible
  //  nested create  | create | checked
  //  nested update  | update | checked
  //  nested connect | -      | not possible
  //  nested disconn | -      | not possible
  //  nested delete  | -      | not possible
  //  nested set     | -      | not possible
  //  nested upsert  | -      | not possible
  //  nested deleteM | -      | not possible
  //  nested updateM } -      | not possible
  "Using a compound id that contains a relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child Child  @relation(references: [id])
         |  name  String
         |  age   Int
         |
         |  @@id([child, name])
         |}
         |
         |model Child {
         |  id      Int @id
         |  name    String
         |  parents Parent[]
         |}
       """
    }
    database.setup(project)

    // Mutations in this test (WIP):
    //  create         | root   | checked
    //  update         | root   | checked
    //  delete         | root   | -
    //  nested connect | create | checked
    //  nested create  | create | checked
    //  nested connect | update | -
    //  nested create  | update | -
    //  nested update  | update | -
    //  nested delete  | update | checked
    //  nested disconn | -      | -
    //  nested set     | -      | -
    val res0 = server.query(
      """
        |mutation {
        |  createChild(
        |     data: {
        |       id: 0,
        |       name: "Peter"
        |     }
        |  ){
        |    id
        |  }
        |}
      """,
      project
    )

    res0.toString() should be("{\"data\":{\"createChild\":{\"id\":0}}}")

    val res1 = server.query(
      """
        |mutation {
        |  createParent(
        |    data: {
        |      name: "Parker",
        |      age: 10000,
        |      child: {
        |        connect: { id: 0 }
        |      }
        |    }
        |  ){
        |    age
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"age\":10000}}}")

    val res2 = server.query(
      """
        |mutation {
        |  createChild(
        |    data: {
        |      id: 1,
        |      name: "Paul",
        |      parents: {
        |        create: [
        |          { name: "Panther", age: 10 },
        |          { name: "Pawlowski", age: 100 },
        |          { name: "Parker", age: 1000 }
        |        ]
        |      }
        |    }
        |  ){
        |    parents { age }
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("{\"data\":{\"createChild\":{\"parents\":[{\"age\":10},{\"age\":1000},{\"age\":100}]}}}")

    val res3 = server.query(
      """
        |mutation {
        |  updateParent(
        |     where: { child_name: { child: 1, name: "Panther" }}
        |     data: { age: 12 }
        |  ){
        |    age
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateParent\":{\"age\":12}}}")

    val res4 = server.query(
      """
        |mutation {
        |  updateChild(
        |    where: { id: 1 }
        |    data: {
        |      parents: {
        |        update: {
        |          where: { child_name: { child: 1, name: "Panther" } }
        |          data: { age: 12 }
        |        }
        |      }
        |    }
        |  ) {
        |    parents {
        |      age
        |    }
        |  }
        |}
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"updateChild\":{\"parents\":[{\"age\":12},{\"age\":1000},{\"age\":100}]}}}")

    val res6 = server.query(
      """
        |mutation {
        |  updateChild(
        |    where: { id: 1 }
        |    data: {
        |      parents: {
        |        delete: { child_name: { child: 1, name: "Panther" } }
        |      }
        |    }
        |  ) {
        |    parents {
        |      age
        |    }
        |  }
        |}
      """,
      project
    )

    res6.toString() should be("{\"data\":{\"updateChild\":{\"parents\":[{\"age\":1000},{\"age\":100}]}}}")
  }
}
