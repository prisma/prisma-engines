package writes.ids.relation_pks

import org.scalatest.{FlatSpec, Matchers}
import util._

// RS: Ported
// Note: These tests changed from including the relation fields into only including the scalars as per the new relations
// implementation. Tests are retained as they offer a good coverage over scalar + relation field usage.
//
// 1) Checks if relation fields in @@id in any constellation work with our mutations.
// Possible relation cardinalities:
// - 1!:1!
// - 1!:1
// - 1!:M
//
// 2) Checks basic cursor functionality.
class CompoundPKRelationFieldSpec extends FlatSpec with Matchers with ApiSpecBase {
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
  "Using a compound ID that includes a 1!:1 single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name     String
         |  age      Int
         |  child_id Int
         |
         |  child Child  @relation(fields: [child_id], references: [id])
         |  @@id([name, child_id])
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
        |  updateParent(
        |    where: {
        |      name_child_id: {
        |        name: "Paul"
        |        child_id: 1
        |      }
        |    }
        |    data: { age: { set: 41 }}
        |  ) {
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
        |  updateChild(where: { id: 1 } data: { parent: { update: { age: { set: 42 }}}}) {
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
        |    where: {
        |      name_child_id: {
        |        name: "Paul"
        |        child_id: 2
        |      }
        |    }
        |    update: { name: { set: "Milutin" }, age: { set: 43 }}
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
        |    where: {
        |      name_child_id: {
        |        name: "Milutin"
        |        child_id: 2
        |      }
        |    }
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
        |            name: { set: "doesn't matter" }
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
  //  nested disconn | -      | not possible (1!:1)
  //  nested set     | -      | not possible (1!:1)
  //  nested deleteM | -      | not possible (1!:1)
  //  nested updateM | -      | not possible (1!:1)
  "Using a compound ID that includes a 1!:1 multi-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name      String
         |  age       Int
         |  child_id  Int
         |  child_ssn String
         |
         |  child Child  @relation(fields: [child_id, child_ssn], references: [id, ssn])
         |  @@id([name, child_id, child_ssn])
         |}
         |
         |model Child {
         |  id     Int    @id
         |  ssn    String @unique
         |  name   String
         |  parent Parent?
         |
         |  @@unique([id, ssn])
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, ssn: "1", name: "Panther" }}}) {
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
        |  updateParent(
        |    where: {
        |      name_child_id_child_ssn: {
        |        name: "Paul"
        |        child_id: 1
        |        child_ssn: "1"
        |      }}
        |      data: { age: { set: 41 }}
        |    ) {
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
        |  updateChild(where: { id: 1 } data: { parent: { update: { age: { set: 42 }}}}) {
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
        |    where: {
        |      name_child_id_child_ssn: {
        |        name: "Paul"
        |        child_id: 2
        |        child_ssn: "2"
        |      }
        |    }
        |    update: { name: { set: "Milutin" }, age: { set: 43 }}
        |    create: { name: "Milutin", age: 43, child: { create: { id: 2, ssn: "2", name: "Nikola" } } }
        |  ) {
        |    age
        |    child {
        |      id
        |      ssn
        |    }
        |  }
        |}
        |
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"age\":43,\"child\":{\"id\":2,\"ssn\":\"2\"}}}}")

    val res5 = server.query(
      """
        |mutation {
        |  deleteParent(
        |    where: {
        |      name_child_id_child_ssn: {
        |        name: "Milutin"
        |        child_id: 2
        |        child_ssn: "2"
        |      }
        |    }
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
        |      ssn
        |    }
        |  }
        |}
      """,
      project
    )

    res6.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Milutin\",\"child\":{\"id\":2,\"ssn\":\"2\"}}}}")

    val res7 = server.query(
      """
        |mutation {
        |  createChild(
        |    data: {
        |      id: 3
        |      ssn: "3"
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
        |            name: { set: "doesn't matter" }
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
  "Using a compound ID that includes a 1!:M single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name     String
         |  age      Int
         |  child_id Int
         |
         |  child Child  @relation(fields: [child_id], references: [id])
         |  @@id([name, child_id])
         |}
         |
         |model Child {
         |  id      Int      @id
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
        |  updateParent(
        |    where: {
        |      name_child_id: {
        |        name: "Paul"
        |        child_id: 1
        |      }
        |    }
        |    data: { age: { set: 41 }}
        |  ) {
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
        |         where: { age: { equals: 41 }}
        |         data: { age: { set: 42 }}
        |       }
        |     }
        |  }) {
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
        |    where: {
        |      name_child_id: {
        |        name: "Paul"
        |        child_id: 2
        |      }
        |    }
        |    update: { name: { set: "Milutin" }, age: { set: 43 }}
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
        |    where: {
        |      name_child_id: {
        |        name: "Milutin"
        |        child_id: 2
        |      }
        |    }
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
        |          where: {
        |            name_child_id: {
        |              name: "Đuka"
        |              child_id: 3
        |            }
        |          }
        |          create: { name: "Đuka", age: 40 }
        |          update: { name: { set: "doesn't matter" }}
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
        |          where: { age: { equals: 40 }}
        |          data: { age: { set: 41 }}
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
        |          age: { equals: 41 }
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
  "Using a compound ID that includes a 1!:M multi-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name      String
         |  age       Int
         |  child_id  Int
         |  child_ssn String
         |
         |  child Child  @relation(fields: [child_id, child_ssn], references: [id, ssn])
         |  @@id([name, child_id, child_ssn])
         |}
         |
         |model Child {
         |  id      Int    @id
         |  ssn     String @unique
         |  name    String
         |  parents Parent[]
         |
         |  @@unique([id, ssn])
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul", age: 40, child: { create: { id: 1, ssn: "1", name: "Panther" }}}) {
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
        |  updateParent(
        |    where: {
        |      name_child_id_child_ssn: {
        |        name: "Paul"
        |        child_id: 1
        |        child_ssn: "1"
        |      }
        |    }
        |    data: { age: { set: 41 }}
        |  ) {
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
        |  updateChild(
        |    where: { id: 1 }
        |    data: {
        |      parents: {
        |         updateMany: {
        |           where: { age: { equals: 41 }}
        |           data: { age: { set: 42 }}
        |         }
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
        |    where: {
        |      name_child_id_child_ssn: {
        |        name: "Paul"
        |        child_id: 2
        |        child_ssn: "2"
        |      }
        |    }
        |    update: { name: { set: "Milutin" }, age: { set: 43 }}
        |    create: { name: "Milutin", age: 43, child: { create: { id: 2, ssn: "2", name: "Nikola" } } }
        |  ) {
        |    age
        |    child {
        |      id
        |      ssn
        |    }
        |  }
        |}
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"age\":43,\"child\":{\"id\":2,\"ssn\":\"2\"}}}}")

    val res5 = server.query(
      """
        |mutation {
        |  deleteParent(
        |    where: {
        |      name_child_id_child_ssn: {
        |        name: "Milutin"
        |        child_id: 2
        |        child_ssn: "2"
        |      }
        |    }
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
        |      ssn
        |    }
        |  }
        |}
      """,
      project
    )

    res6.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Milutin\",\"child\":{\"id\":2,\"ssn\":\"2\"}}}}")

    val res7 = server.query(
      """
        |mutation {
        |  createChild(
        |    data: {
        |      id: 3
        |      ssn: "3"
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
        |          where: {
        |            name_child_id_child_ssn: {
        |              name: "Đuka"
        |              child_id: 3
        |              child_ssn: "3"
        |            }
        |          }
        |          create: { name: "Đuka", age: 40 }
        |          update: { name: { set: "doesn't matter" }}
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
        |          where: { age: { equals: 40 }}
        |          data: { age: { set: 41 }}
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
        |          age: { equals: 41 }
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
