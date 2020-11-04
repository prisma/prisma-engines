package writes.unchecked_writes

import org.scalatest.{FlatSpec, Matchers}
import util._

// Important: This test covers ALL nested create inputs, like create nested, connectOrCreate, nested upsert,
// because schema building uses the exact same types under the hood.
class UncheckedNestedCreateSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Unchecked nested creates" should "allow writing non-parent inlined relation scalars" in {
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

    // B can't be written because it's the parent.
    server.queryThatMustFail(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: 1
        |    uniq_2: 1
        |    a: {
        |      create: {
        |        id: 1
        |        b_id_1: 123,
        |        b_id_2: 321,
        |      }
        |    }
        |  }) {
        |    uniq_1
        |    uniq_2
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      legacy = false
    )

    //
    var res = server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: 2
        |    uniq_2: 2
        |    a: {
        |      create: {
        |        id: 2
        |      }
        |    }
        |  }) {
        |    a {
        |      b {
        |       uniq_1
        |       uniq_2
        |      }
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelB":{"a":[{"b":{"uniq_1":2,"uniq_2":2}}]}}}""")

    res = server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: 3
        |    uniq_2: 3
        |    a: {
        |      create: {
        |        id: 3
        |        c_id_1: null
        |        c_id_2: 123
        |      }
        |    }
        |  }) {
        |    a {
        |      b {
        |       uniq_1
        |       uniq_2
        |      }
        |      c {
        |        uniq_1
        |        uniq_2
        |      }
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelB":{"a":[{"b":{"uniq_1":3,"uniq_2":3},"c":null}]}}}""")
  }

  "Unchecked nested creates" should "fail if required relation scalars are not provided" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id     Int  @id
         |  b_id_1 Int
         |  b_id_2 Int
         |  c_id_1 Int
         |  c_id_2 Int
         |
         |  b ModelB @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
         |  c ModelC @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
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

    server.queryThatMustFail(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: 1
        |    uniq_2: 1
        |    a: {
        |      create: {
        |        id: 1
        |        c_id_1: 123,
        |      }
        |    }
        |  }) {
        |    uniq_1
        |    uniq_2
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2009,
      errorContains =
        """`Mutation.createOneModelB.data.ModelBUncheckedCreateInput.a.ModelAUncheckedCreateManyWithoutBInput.create.ModelAUncheckedCreateWithoutBInput.c_id_2`: A value is required but not set.""",
      legacy = false
    )
  }

  "Unchecked nested creates" should "not allow writing inlined relations regularly" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int  @id
         |  b_id Int
         |  c_id Int
         |  d_id Int
         |
         |  b ModelB @relation(fields: [b_id], references: [id])
         |  c ModelC @relation(fields: [c_id], references: [id])
         |  d ModelD @relation(fields: [d_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int    @id
         |  a  ModelA
         |}
         |
         |model ModelC {
         |  id Int    @id
         |  a  ModelA
         |}
         |
         |model ModelD {
         |  id Int    @id
         |  a  ModelA
         |}
      """
    }

    database.setup(project)

    // We need ModelD to trigger the correct input. We're coming from B, so B is out,
    // then we use C to trigger the union on the unchecked type, then we use d as a regular
    // relation in the input that must fail.
    server.queryThatMustFail(
      """
        |mutation {
        |  createOneModelB(data: {
        |    id: 1
        |    a: {
        |      create: {
        |        id: 1
        |        c_id: 1
        |        d: {
        |          create: { id: 1 }
        |        }
        |       }
        |     }
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

  "Unchecked nested creates" should "allow writing non-parent, non-inlined relations normally" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int @id
         |  b_id Int
         |  d_id Int
         |
         |  b ModelB @relation(fields: [b_id], references: [id])
         |  c ModelC
         |  d ModelD @relation(fields: [d_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int    @id
         |  a  ModelA
         |}
         |
         |model ModelC {
         |  id   Int    @id
         |  a_id Int 
         |  a    ModelA @relation(fields: [a_id], references: [id])
         |}
         |
         |model ModelD {
         |  id Int     @id
         |  a  ModelA?
         |}
      """
    }

    database.setup(project)

    server.query(
      """
        |mutation {
        |  createOneModelD(data: {
        |    id: 1
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
        |  createOneModelB(data: {
        |    id: 1
        |    a: {
        |      create: {
        |        id: 1,
        |        d_id: 1
        |        c: { create: { id: 1 }}
        |      }
        |    }
        |  }) {
        |    a {
        |      c { id }
        |      d { id }
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelB":{"a":{"c":{"id":1},"d":{"id":1}}}}}""")
  }

  "Unchecked nested creates" should "honor defaults and make required relation scalars optional" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id
         |  b_id Int    
         |  c_id Int    @default(1)
         |  b    ModelB @relation(fields: [b_id], references: [id])
         |  c    ModelC @relation(fields: [c_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int    @id
         |  a  ModelA
         |}
         |
         |model ModelC {
         |  id Int      @id
         |  a  ModelA[]
         |}
      """
    }

    database.setup(project)

    server.query(
      """
        |mutation {
        |  createOneModelC(data: {
        |    id: 1
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
        |  createOneModelB(data: {
        |    id: 1
        |    a: { create: { id: 1 }}
        |  }) {
        |    a { c { id }}
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelB":{"a":{"c":{"id":1}}}}}""")
  }

  "Unchecked nested creates" should "allow to write to autoincrement IDs directly" taggedAs IgnoreMsSql in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id @default(autoincrement())
         |  b_id Int
         |  b    ModelB @relation(fields: [b_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int    @id
         |  a  ModelA
         |}
      """
    }

    database.setup(project)

    val res = server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    id: 1
        |    a: { create: { id: 2 }}
        |  }) {
        |    a { id }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"createOneModelB":{"a":{"id":2}}}}""")
  }
}
