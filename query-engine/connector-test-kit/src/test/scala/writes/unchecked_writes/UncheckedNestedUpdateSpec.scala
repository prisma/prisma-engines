package writes.unchecked_writes

import org.scalatest.{FlatSpec, Matchers}
import util._

// Important: This test covers ALL nested create inputs, like create nested, connectOrCreate, nested upsert,
// because schema building uses the exact same types under the hood.
class UncheckedNestedUpdateSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Unchecked nested updates" should "allow writing non-parent inlined relation scalars" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id     Int  @id
         |  b_id_1 String
         |  b_id_2 String
         |  c_id_1 String?
         |  c_id_2 String?
         |
         |  b ModelB  @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
         |  c ModelC? @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
         |}
         |
         |model ModelB {
         |  uniq_1    String
         |  uniq_2    String
         |  
         |  a ModelA?
         |
         |  @@unique([uniq_1, uniq_2])
         |}
         |
         |model ModelC {
         |  uniq_1    String
         |  uniq_2    String
         |  
         |  a ModelA?
         |
         |  @@unique([uniq_1, uniq_2])
         |}
      """
    }

    database.setup(project)

    // Setup
    server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
        |    b: { create: { uniq_1: "b1_1", uniq_2: "b1_2" }}
        |    c: { create: { uniq_1: "c1_1", uniq_2: "c1_2" }}
        |  }) {
        |    id
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
        |    uniq_1: "c2_1"
        |    uniq_2: "c2_2"
        |  }) {
        |    uniq_1
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    // C can be updated for A
    var res = server.query(
      """
        |mutation {
        |  updateOneModelB(where: {
        |    uniq_1_uniq_2: {
        |      uniq_1: "b1_1"
        |      uniq_2: "b1_2"
        |    }
        |  }, data: {
        |    a: {
        |      update: {
        |        c_id_1: "c2_1"
        |        c_id_2: "c2_2"
        |      }
        |    }
        |  }) {
        |    a {
        |      c {
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

    res.toString() should be("""{"data":{"updateOneModelB":{"a":{"c":{"uniq_1":"c2_1","uniq_2":"c2_2"}}}}}""")

    res = server.query(
      """
        |mutation {
        |  updateOneModelB(where: {
        |    uniq_1_uniq_2: {
        |      uniq_1: "b1_1"
        |      uniq_2: "b1_2"
        |    }
        |  }, data: {
        |    a: {
        |      update: {
        |        c_id_1: null
        |      }
        |    }
        |  }) {
        |    a {
        |      c {
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

    res.toString() should be("""{"data":{"updateOneModelB":{"a":{"c":null}}}}""")
  }

  "Unchecked nested updates" should "not allow writing parent inlined relation scalars" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id
         |  b_id Int
         |  b    ModelB @relation(fields: [b_id], references: [id])
         |}
         |
         |model ModelB {
         |  id Int      @id
         |  a  ModelA
         |}
         """
    }

    database.setup(project)

    // B can't be written because it's the parent.
    server.queryThatMustFail(
      """
        |mutation {
        |  updateOneModelB(where: { id: 1 }, data: {
        |    a: {
        |      update: {
        |        b_id: 123
        |      }
        |    }
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

  "Unchecked nested updates" should "not allow writing inlined relations regularly" in {
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
        |  updateOneModelB(data: {
        |    a: {
        |      update: {
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

  "Unchecked nested updates" should "allow writing non-parent, non-inlined relations normally" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int  @id
         |  b_id Int
         |  d_id Int
         |
         |  b ModelB  @relation(fields: [b_id], references: [id])
         |  c ModelC?
         |  d ModelD  @relation(fields: [d_id], references: [id])
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
        |  createOneModelA(data: {
        |    id: 1
        |    b: { create: { id: 1 } }
        |    d: { create: { id: 1 } }
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    server.query(
      """
        |mutation {
        |  createOneModelD(data: {
        |    id: 2
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
        |  updateOneModelB(where: { id: 1 }, data: {
        |    a: {
        |      update: {
        |        d_id: 2
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

    res.toString() should be("""{"data":{"updateOneModelB":{"a":{"c":{"id":1},"d":{"id":2}}}}}""")
  }

  "Unchecked nested updates" should "allow to write to autoincrement IDs directly" taggedAs IgnoreMsSql in {
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

    server
      .query(
        """
        |mutation {
        |  createOneModelA(data: {
        |    b: { create: { id: 1 }}
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
        |  updateOneModelB(where: { id: 1 }, data: {
        |    a: { update: { id: 111 }}
        |  }) {
        |    a { id }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateOneModelB":{"a":{"id":111}}}}""")
  }
}
