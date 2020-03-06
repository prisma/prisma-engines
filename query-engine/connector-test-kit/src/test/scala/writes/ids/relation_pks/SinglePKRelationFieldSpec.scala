package writes.ids.relation_pks

import org.scalatest.{FlatSpec, Matchers}
import util._

// Checks if relation fields in @id in any constellation work with our mutations.
// Possible relation cardinalities:
// - 1!:1!
// - 1!:1
// - 1!:M
class SinglePKRelationFieldSpec extends FlatSpec with Matchers with ApiSpecBase {
  //todo @@Id
  // single id is also a relation
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
  "Using a simple id that is also a 1!:1! relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child Child  @relation(references: [id]) @id
         |  name  String
         |  age   Int
         |}
         |
         |model Child {
         |  id      Int    @id
         |  name    String
         |  parent  Parent
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 40, child: { create: {id: 1, name: "Panther" }}}) {
        |    name
        |    age
        |    child{
        |       id
        |       name
        |    }
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Paul\",\"age\":40,\"child\":{\"id\":1,\"name\":\"Panther\"}}}}")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(where: { child: 1 } data: { age: 41 }) {
        |    name
        |    age
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("{\"data\":{\"updateParent\":{\"name\":\"Paul\",\"age\":41}}}")

    val res3 = server.query(
      """
        |mutation {
        |  updateChild(where: { id: 1 } data: { parent: { update: { age: 42 }}}) {
        |    parent { age }
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateChild\":{\"parent\":{\"age\":42}}}}")

    val res4 = server.query(
      """
        |mutation {
        |  upsertParent(
        |    where: { child: 2 }
        |    update: { age: 43 }
        |    create: { name: "Milutin", age: 42, child: { create: { id: 2, name: "Nikola" } } }
        |  ) {
        |    age
        |  }
        |}
        |
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"age\":43}}}")
  }

  // Mutations in this test:
  //  create         | root   | checked
  //  update         | root   | checked
  //  delete         | root   | checked
  //  upsert         | root   | checked
  //  updateMany     | root   | unnecessary
  //  deleteMany     | root   | unnecessary
  //  nested create  | create | checked
  //  nested update  | update | checked
  //  nested connect | create | checked
  //  nested connect | update | checked
  //  nested delete  | -      | checked
  //  nested upsert  | update | checked
  //  nested disconn | -      | not possible (1!:1)
  //  nested set     | -      | not possible (1!:1)
  //  nested deleteM | -      | not possible (1!:1)
  //  nested updateM | -      | not possible (1!:1)
  "Using a simple id that is also a 1!:1 relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child Child  @relation(references: [id]) @id
         |  name  String
         |  age   Int
         |}
         |
         |model Child {
         |  id      Int    @id
         |  name    String
         |  parent  Parent?
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, name: "Panther" }}}) {
        |    name
        |    age
        |    child{
        |       id
        |       name
        |    }
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Paul\",\"age\":40,\"child\":{\"id\":1,\"name\":\"Panther\"}}}}")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(where: { child: 1 } data: { age: 41 }) {
        |    name
        |    age
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("{\"data\":{\"updateParent\":{\"name\":\"Paul\",\"age\":41}}}")

    val res3 = server.query(
      """
        |mutation {
        |  updateChild(where: { id: 1 } data: { parent: { update: { age: 42 }}}) {
        |    parent { age }
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateChild\":{\"parent\":{\"age\":42}}}}")

    val res4 = server.query(
      """
        |mutation {
        |  upsertParent(
        |    where: { child: 2 }
        |    update: { age: 43 }
        |    create: { name: "Milutin", age: 43, child: { create: { id: 2, name: "Nikola" } } }
        |  ) {
        |    age
        |  }
        |}
        |
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"age\":43}}}")

    val res5 = server.query(
      """
        |mutation {
        |  deleteParent(
        |    where: { child: 2 }
        |  ) {
        |    name
        |  }
        |}
        |
      """,
      project
    )

    res5.toString() should be("{\"data\":{\"deleteParent\":{\"name\":\"Milutin\"}}}")

    val res6 = server.query(
      """
        |mutation {
        |  createParent(
        |    data: {
        |      name: "Milutin",
        |      age: 43
        |      child: {
        |        connect: {
        |          id: 2
        |        }
        |      }
        |    }
        |  ) {
        |    name
        |    child {
        |      id
        |    }
        |  }
        |}
        |
      """,
      project
    )

    res6.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Milutin\",\"child\":{\"id\":2}}}}")

    val res7 = server.query(
      """
        |mutation {
        |  createChild(
        |    data: {
        |      id: 3
        |      name: "Angelina",
        |    }
        |  ) {
        |    id
        |  }
        |}
        |
      """,
      project
    )

    res7.toString() should be("{\"data\":{\"createChild\":{\"id\":3}}}")

    // Currently doesnt work
//    val res8 = server.query(
//      """
//        |mutation {
//        |  updateParent(
//        |    where: { child: 2 }
//        |    data: {
//        |      child: {
//        |        connect: {
//        |          id: 3
//        |        }
//        |      }
//        |    }
//        |  ) {
//        |    child {
//        |      id
//        |    }
//        |  }
//        |}
//      """,
//      project
//    )
//
//    res8.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Milutin\",\"child\":{\"id\":2}}}}")

    val res9 = server.query(
      """
        |mutation {
        |  updateChild(
        |    where: { id: 3 }
        |    data: {
        |      parent: {
        |        upsert: {
        |          create: {
        |            name: "Đuka",
        |            age: 40
        |          }
        |          update: {
        |            name: "doesn't matter"
        |          }
        |        }
        |      }
        |    }
        |  ) {
        |    id
        |    parent {
        |      child {
        |        id
        |      }
        |    }
        |  }
        |}
      """,
      project
    )

    res9.toString() should be("{\"data\":{\"updateChild\":{\"id\":3,\"parent\":{\"child\":{\"id\":3}}}}}")

    val res10 = server.query(
      """
        |mutation {
        |  updateChild(
        |    where: { id: 3 }
        |    data: {
        |      parent: {
        |        delete: true
        |      }
        |    }
        |  ) {
        |    id
        |    parent {
        |      child {
        |        id
        |      }
        |    }
        |  }
        |}
      """,
      project
    )

    res10.toString() should be("{\"data\":{\"updateChild\":{\"id\":3,\"parent\":null}}}")
  }

  // Mutations in this test:
  //  create         | root   | checked
  //  update         | root   | checked
  //  delete         | root   | checked
  //  upsert         | root   | checked
  //  updateMany     | root   | unnecessary
  //  deleteMany     | root   | unnecessary
  //  nested create  | create | checked
  //  nested update  | update | checked
  //  nested connect | create | checked
  //  nested connect | update | checked
  //  nested delete  | -      | checked
  //  nested upsert  | update | checked
  //  nested deleteM | -      | checked
  //  nested updateM | -      | checked
  //  nested disconn | -      | not possible (1!:m)
  //  nested set     | -      | not (really) possible (1!:m)
  "Using a simple id that is also a 1!:M relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child Child  @relation(references: [id]) @id
         |  name  String
         |  age   Int
         |}
         |
         |model Child {
         |  id      Int    @id
         |  name    String
         |  parents Parent[]
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, name: "Panther" }}}) {
        |    name
        |    age
        |    child {
        |       id
        |       name
        |    }
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Paul\",\"age\":40,\"child\":{\"id\":1,\"name\":\"Panther\"}}}}")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(where: { child: 1 } data: { age: 41 }) {
        |    name
        |    age
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("{\"data\":{\"updateParent\":{\"name\":\"Paul\",\"age\":41}}}")

    val res3 = server.query(
      """
        |mutation {
        |  updateChild(where: { id: 1 } data: {
        |    parents: {
        |       updateMany: {
        |         where: { age: 41 }
        |         data: { age: 42 } }
        |       }
        |     }
        |  ) {
        |    parents { name age }
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateChild\":{\"parents\":[{\"name\":\"Paul\",\"age\":42}]}}}")

    val res4 = server.query(
      """
        |mutation {
        |  upsertParent(
        |    where: { child: 2 }
        |    update: { age: 43 }
        |    create: { name: "Milutin", age: 43, child: { create: { id: 2, name: "Nikola" } } }
        |  ) {
        |    age
        |  }
        |}
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"age\":43}}}")

    val res5 = server.query(
      """
        |mutation {
        |  deleteParent(
        |    where: { child: 2 }
        |  ) {
        |    name
        |  }
        |}
      """,
      project
    )

    res5.toString() should be("{\"data\":{\"deleteParent\":{\"name\":\"Milutin\"}}}")

    val res6 = server.query(
      """
        |mutation {
        |  createParent(
        |    data: {
        |      name: "Milutin",
        |      age: 43
        |      child: {
        |        connect: {
        |          id: 2
        |        }
        |      }
        |    }
        |  ) {
        |    name
        |    child {
        |      id
        |    }
        |  }
        |}
      """,
      project
    )

    res6.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Milutin\",\"child\":{\"id\":2}}}}")

    val res7 = server.query(
      """
        |mutation {
        |  createChild(
        |    data: {
        |      id: 3
        |      name: "Angelina",
        |    }
        |  ) {
        |    id
        |  }
        |}
        |
      """,
      project
    )

    res7.toString() should be("{\"data\":{\"createChild\":{\"id\":3}}}")

    // Currently doesnt work
    //    val res8 = server.query(
    //      """
    //        |mutation {
    //        |  updateParent(
    //        |    where: { child: 2 }
    //        |    data: {
    //        |      child: {
    //        |        connect: {
    //        |          id: 3
    //        |        }
    //        |      }
    //        |    }
    //        |  ) {
    //        |    child {
    //        |      id
    //        |    }
    //        |  }
    //        |}
    //      """,
    //      project
    //    )
    //
    //    res8.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Milutin\",\"child\":{\"id\":2}}}}")

    val res9 = server.query(
      """
        |mutation {
        |  updateChild(
        |    where: { id: 3 }
        |    data: {
        |      parents: {
        |        upsert: {
        |          where: { child: 3 }
        |          create: { name: "Đuka", age: 40 }
        |          update: { name: "doesn't matter" }
        |        }
        |      }
        |    }
        |  ) {
        |    id
        |    parents {
        |      child {
        |        id
        |      }
        |    }
        |  }
        |}
      """,
      project
    )

    res9.toString() should be("{\"data\":{\"updateChild\":{\"id\":3,\"parents\":[{\"child\":{\"id\":3}}]}}}")

    val res10 = server.query(
      """
        |mutation {
        |  updateChild(
        |    where: { id: 3 }
        |    data: {
        |      parents: {
        |        updateMany: {
        |          where: { age: 40 }
        |          data: { age: 41 }
        |        }
        |      }
        |    }
        |  ) {
        |    id
        |    parents {
        |      age
        |      child {
        |        id
        |      }
        |    }
        |  }
        |}
      """,
      project
    )

    res10.toString() should be("{\"data\":{\"updateChild\":{\"id\":3,\"parents\":[{\"age\":41,\"child\":{\"id\":3}}]}}}")

    val res11 = server.query(
      """
       |mutation {
       |  updateChild(
       |    where: { id: 3 }
       |    data: {
       |      parents: {
       |        deleteMany: {
       |          age: 41
       |        }
       |      }
       |    }
       |  ) {
       |    id
       |    parents {
       |      child {
       |        id
       |      }
       |    }
       |  }
       |}
      """,
      project
    )

    res11.toString() should be("{\"data\":{\"updateChild\":{\"id\":3,\"parents\":[]}}}")

  }
}
