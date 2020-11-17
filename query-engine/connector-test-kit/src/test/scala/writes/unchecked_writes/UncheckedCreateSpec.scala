package writes.unchecked_writes

import org.scalatest.{FlatSpec, Matchers}
import util._

// Important: This test covers ALL top level create inputs, like create & upsert,
// because schema building uses the exact same types under the hood.
class UncheckedCreateSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Unchecked creates" should "allow writing inlined relation scalars" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id     Int  @id
         |  b_id_1 Int
         |  b_id_2 Int
         |  c_id_1 Int?
         |  c_id_2 Int?
         |
         |  b ModelB  @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
         |  c ModelC? @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
         |}
         |
         |model ModelB {
         |  uniq_1    Int
         |  uniq_2    Int
         |  
         |  a ModelA[]
         |
         |  @@unique([uniq_1, uniq_2])
         |}
         |
         |model ModelC {
         |  uniq_1    Int
         |  uniq_2    Int
         |  
         |  a ModelA[]
         |
         |  @@unique([uniq_1, uniq_2])
         |}
         """
    }

    database.setup(project)

    // Ensure inserted foreign keys for A are valid.
    server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: 11
        |    uniq_2: 12
        |  }) {
        |    uniq_1
        |    uniq_2
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    server.query(
      """
        |mutation {
        |  createOneModelC(data: {
        |    uniq_1: 21
        |    uniq_2: 22
        |  }) {
        |    uniq_1
        |    uniq_2
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    var res = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |    b_id_1: 11
        |    b_id_2: 12
        |    c_id_1: 21
        |    c_id_2: 22
        |  }) {
        |    id
        |    b {
        |      uniq_1
        |      uniq_2
        |    }
        |    c {
        |      uniq_1
        |      uniq_2
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelA":{"id":1,"b":{"uniq_1":11,"uniq_2":12},"c":{"uniq_1":21,"uniq_2":22}}}}""")

    res = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 2
        |    b_id_1: 11
        |    b_id_2: 12
        |    c_id_1: null
        |    c_id_2: 22
        |  }) {
        |    id
        |    b {
        |      uniq_1
        |      uniq_2
        |    }
        |    c {
        |      uniq_1
        |      uniq_2
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelA":{"id":2,"b":{"uniq_1":11,"uniq_2":12},"c":null}}}""")
  }

  "Unchecked creates" should "not allow writing inlined relations regularly" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int  @id
         |  b_id Int
         |  c_id Int?
         |
         |  b ModelB  @relation(fields: [b_id], references: [id])
         |  c ModelC? @relation(fields: [c_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int     @id
         |  a  ModelA?
         |}
         |
         |model ModelC {
         |  id Int     @id
         |  a  ModelA?
         |}
      """
    }

    database.setup(project)

    server.queryThatMustFail(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |    b_id: 11
        |    c: { create: { id: 21 } }
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      legacy = false
    )
  }

  "Unchecked creates" should "require to write required relation scalars and must allow optionals to be omitted" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int  @id
         |  b_id Int
         |  c_id Int?
         |
         |  b ModelB  @relation(fields: [b_id], references: [id])
         |  c ModelC? @relation(fields: [c_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int     @id
         |  a  ModelA?
         |}
         |
         |model ModelC {
         |  id Int     @id
         |  a  ModelA?
         |}
      """
    }

    database.setup(project)

    server.queryThatMustFail(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      errorContains = "`Mutation.createOneModelA.data.ModelAUncheckedCreateInput.b_id`: A value is required but not set.",
      legacy = false
    )

    server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    id: 11
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    val res = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |    b_id: 11
        |  }) {
        |    id
        |    b { id }
        |    c { id }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelA":{"id":1,"b":{"id":11},"c":null}}}""")
  }

  "Unchecked creates" should "allow writing non-inlined relations normally" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int @id
         |  b_id Int
         |  b ModelB  @relation(fields: [b_id], references: [id])
         |  c ModelC?
         |}
         |
         |model ModelB {
         |  id Int     @id
         |  a  ModelA?
         |}
         |
         |model ModelC {
         |  id   Int    @id
         |  a_id Int 
         |  a    ModelA @relation(fields: [a_id], references: [id])
         |}
      """
    }

    database.setup(project)

    server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    id: 11
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    val res = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |    b_id: 11
        |    c: { create: { id: 21 }}
        |  }) {
        |    id
        |    b { id }
        |    c { id }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelA":{"id":1,"b":{"id":11},"c":{"id":21}}}}""")
  }

  "Unchecked creates" should "honor defaults and make required relation scalars optional" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id
         |  b_id Int    @default(11)
         |  b    ModelB @relation(fields: [b_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int      @id
         |  a  ModelA[]
         |}
      """
    }

    database.setup(project)

    server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    id: 11
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    val res = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |  }) {
        |    b { id }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelA":{"b":{"id":11}}}}""")
  }

  "Unchecked creates" should "allow to write to autoincrement IDs directly" taggedAs IgnoreMsSql in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id @default(autoincrement())
         |}
      """
    }

    database.setup(project)

    val res = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 111
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelA":{"id":111}}}""")
  }
}
