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
  "Using a compound ID that includes a 1!:1! single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name  String
         |  child Child  @relation(references: [id])
         |  age   Int
         |
         |  @@id([name, child])
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
        |  updateParent(where: { name_child: {
        |    child: 1
        |    name: "Paul"
        |  } } data: { age: 41 }) {
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
        |    where: { name_child: {
        |      name: "Paul"
        |      child: 2
        |    }}
        |    update: { name: "Milutin", age: 43 }
        |    create: { name: "Milutin", age: 43, child: { create: { id: 2, name: "Nikola" } } }
        |  ) {
        |    age
        |    child {
        |      id
        |    }
        |  }
        |}
        |
      """,
      project
    )

    res4.toString() should be("{\"data\":{\"upsertParent\":{\"age\":43,\"child\":{\"id\":2}}}}")
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
  "Using an ID that is also a 1!:1! multi-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name  String
         |  child Child  @relation(references: [id, ssn])
         |  age   Int
         |
         |  @@id([name, child])
         |}
         |
         |model Child {
         |  id     Int    @id
         |  ssn    String @unique
         |  name   String
         |  parent Parent
         |
         |  @@unique([id, ssn])
         |}
       """
    }
    database.setup(project)

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 40, child: { create: { id: 1, name: "Panther", ssn: "1" }}}) {
        |    name
        |    age
        |    child {
        |       id
        |       name
        |       ssn
        |    }
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Paul\",\"age\":40,\"child\":{\"id\":1,\"name\":\"Panther\",\"ssn\":\"1\"}}}}")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(
        |    where: {
        |      name_child: {
        |        child: { child_id: 1, child_ssn: "1" }
        |        name: "Paul"
        |      }
        |    }
        |    data: { age: 41 }
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
        |  updateChild(where: { id: 1 } data: { parent: { update: { age: 42 }}}) {
        |    parent {
        |      age
        |      child {
        |        id
        |        ssn
        |      }
        |    }
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("{\"data\":{\"updateChild\":{\"parent\":{\"age\":42,\"child\":{\"id\":1,\"ssn\":\"1\"}}}}}")

    val res4 = server.query(
      """
        |mutation {
        |  upsertParent(
        |    where: {
        |      name_child: {
        |        name: "Paul"
        |        child: { child_id: 2, child_ssn: "2" }
        |      }
        |    }
        |    update: { age: 43 }
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
  "Using a compound ID that includes a 1!:1 single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name  String
         |  child Child  @relation(references: [id])
         |  age   Int
         |
         |  @@id([name, child])
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
        |      name_child: {
        |        name: "Paul"
        |        child: 1
        |      }
        |    }
        |    data: { age: 41 }
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
        |    where: {
        |      name_child: {
        |        name: "Paul"
        |        child: 2
        |      }
        |    }
        |    update: { name: "Milutin", age: 43 }
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
        |      name_child: {
        |        name: "Milutin"
        |        child: 2
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
  //  nested disconn | -      | not possible (1!:1)
  //  nested set     | -      | not possible (1!:1)
  //  nested deleteM | -      | not possible (1!:1)
  //  nested updateM | -      | not possible (1!:1)
  "Using a compound ID that includes a 1!:1 multi-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name  String
         |  child Child  @relation(references: [id, ssn])
         |  age   Int
         |
         |  @@id([name, child])
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
        |      name_child: {
        |        name: "Paul"
        |        child: { child_id: 1, child_ssn: "1" }}
        |      }
        |      data: { age: 41 }
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
        |    where: {
        |      name_child: {
        |        name: "Paul"
        |        child: { child_id: 2, child_ssn: "2" }
        |      }
        |    }
        |    update: { name: "Milutin", age: 43 }
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
        |      name_child: {
        |        name: "Milutin"
        |        child: { child_id: 2, child_ssn: "2" }
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
  "Using a compound ID that includes a 1!:M single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child Child  @relation(references: [id])
         |  name  String
         |  age   Int
         |
         |  @@id([name, child])
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
        |      name_child: {
        |        name: "Paul"
        |        child: 1
        |      }
        |    }
        |    data: { age: 41 }
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
        |    where: {
        |      name_child: {
        |        name: "Paul"
        |        child: 2
        |      }
        |    }
        |    update: { name: "Milutin", age: 43 }
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
        |      name_child: {
        |        name: "Milutin"
        |        child: 2
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
        |            name_child: {
        |              name: "Đuka"
        |              child: 3
        |            }
        |          }
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
         |  name  String
         |  child Child  @relation(references: [id, ssn])
         |  age   Int
         |
         |  @@id([name, child])
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
        |      name_child: {
        |        name: "Paul"
        |        child: { child_id: 1, child_ssn: "1" }
        |      }
        |    }
        |    data: { age: 41 }
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
        |    where: {
        |      name_child: {
        |        name: "Paul"
        |        child: { child_id: 2, child_ssn: "2" }
        |      }
        |    }
        |    update: { name: "Milutin", age: 43 }
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
        |      name_child: {
        |        name: "Milutin"
        |        child: { child_id: 2, child_ssn: "2" }
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
        |            name_child: {
        |              name: "Đuka"
        |              child: { child_id: 3, child_ssn: "3" }
        |            }
        |          }
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

  // ---------------------------------------
  // Basic cursor tests:
  // - Before
  // - After
  // ---------------------------------------
  "Using cursors for a compound ID that includes a 1!:M single-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name  String
         |  child Child  @relation(references: [id])
         |  age   Int
         |
         |  @@id([name, child])
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

    val p1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Parent1" , age: 1, child: { create: { id: 1, name: "Child1" }}}) {
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

    p1.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Parent1\",\"age\":1,\"child\":{\"id\":1,\"name\":\"Child1\"}}}}")

    val p2 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Parent2" , age: 2, child: { create: { id: 2, name: "Child2" }}}) {
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

    p2.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}}}}")

    val p3 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Parent3" , age: 3, child: { create: { id: 3, name: "Child3" }}}) {
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

    p3.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Parent3\",\"age\":3,\"child\":{\"id\":3,\"name\":\"Child3\"}}}}")

    val beforeCursor = server.query(
      """
        |query {
        |  parents(
        |    before: {
        |      name_child: {
        |        name: "Parent3"
        |        child: 3
        |      }
        |    }
        |  ){
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

    beforeCursor.toString() should be(
      "{\"data\":{\"parents\":[{\"name\":\"Parent1\",\"age\":1,\"child\":{\"id\":1,\"name\":\"Child1\"}},{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}}]}}")

    val afterCursor = server.query(
      """
        |query {
        |  parents(
        |    after: {
        |      name_child: {
        |        name: "Parent1"
        |        child: 1
        |      }
        |    }
        |  ){
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

    afterCursor.toString() should be(
      "{\"data\":{\"parents\":[{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}},{\"name\":\"Parent3\",\"age\":3,\"child\":{\"id\":3,\"name\":\"Child3\"}}]}}")

    val beforeAfterCursor = server.query(
      """
        |query {
        |  parents(
        |    after: {
        |      name_child: {
        |        name: "Parent1"
        |        child: 1
        |      }
        |    }
        |    before: {
        |      name_child: {
        |        name: "Parent3"
        |        child: 3
        |      }
        |    }
        |  ){
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

    beforeAfterCursor.toString() should be("{\"data\":{\"parents\":[{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}}]}}")
  }

  // ---------------------------------------
  // Basic cursor tests:
  // - Before
  // - After
  // ---------------------------------------
  "Using cursors for a compound ID that includes a 1!:M multi-field relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  name  String
         |  child Child  @relation(references: [id, ssn])
         |  age   Int
         |
         |  @@id([name, child])
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

    val p1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Parent1" , age: 1, child: { create: { id: 1, ssn: "1", name: "Child1" }}}) {
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

    p1.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Parent1\",\"age\":1,\"child\":{\"id\":1,\"name\":\"Child1\"}}}}")

    val p2 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Parent2" , age: 2, child: { create: { id: 2, ssn: "2", name: "Child2" }}}) {
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

    p2.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}}}}")

    val p3 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Parent3" , age: 3, child: { create: { id: 3, ssn: "3", name: "Child3" }}}) {
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

    p3.toString() should be("{\"data\":{\"createParent\":{\"name\":\"Parent3\",\"age\":3,\"child\":{\"id\":3,\"name\":\"Child3\"}}}}")

    val beforeCursor = server.query(
      """
        |query {
        |  parents(
        |    before: {
        |      name_child: {
        |        name: "Parent3"
        |        child: {
        |          child_id: 3
        |          child_ssn: "3"
        |        }
        |      }
        |    }
        |  ){
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

    beforeCursor.toString() should be(
      "{\"data\":{\"parents\":[{\"name\":\"Parent1\",\"age\":1,\"child\":{\"id\":1,\"name\":\"Child1\"}},{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}}]}}")

    val afterCursor = server.query(
      """
        |query {
        |  parents(
        |    after: {
        |      name_child: {
        |        name: "Parent1"
        |        child: {
        |          child_id: 1
        |          child_ssn: "1"
        |        }
        |      }
        |    }
        |  ){
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

    afterCursor.toString() should be(
      "{\"data\":{\"parents\":[{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}},{\"name\":\"Parent3\",\"age\":3,\"child\":{\"id\":3,\"name\":\"Child3\"}}]}}")

    val beforeAfterCursor = server.query(
      """
        |query {
        |  parents(
        |    after: {
        |      name_child: {
        |        name: "Parent1"
        |        child: {
        |          child_id: 1
        |          child_ssn: "1"
        |        }
        |      }
        |    }
        |    before: {
        |      name_child: {
        |        name: "Parent3"
        |        child: {
        |          child_id: 3
        |          child_ssn: "3"
        |        }
        |      }
        |    }
        |  ){
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

    beforeAfterCursor.toString() should be("{\"data\":{\"parents\":[{\"name\":\"Parent2\",\"age\":2,\"child\":{\"id\":2,\"name\":\"Child2\"}}]}}")
  }
}
