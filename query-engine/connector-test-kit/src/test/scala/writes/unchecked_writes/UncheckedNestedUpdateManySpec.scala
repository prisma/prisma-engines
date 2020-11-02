package writes.unchecked_writes

import org.scalatest.{FlatSpec, Matchers}
import util._

class UncheckedNestedUpdateManySpec extends FlatSpec with Matchers with ApiSpecBase {
  "Unchecked nested many updates" should "allow writing non-parent inlined relation scalars" in {
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
         |  a ModelA[]
         |
         |  @@unique([uniq_1, uniq_2])
         |}
         |
         |model ModelC {
         |  uniq_1    String
         |  uniq_2    String
         |  
         |  a ModelA[]
         |
         |  @@unique([uniq_1, uniq_2])
         |}
      """
    }

    database.setup(project)

    // Setup
    // B1 -> A1 -> C1
    // └---> A2 -> C2
    //             C3
    server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: "b1_1"
        |    uniq_2: "b1_2"
        |    a: {
        |      create: [
        |        { id: 1, c: { create: { uniq_1: "c1_1", uniq_2: "c1_2" }}},
        |        { id: 2, c: { create: { uniq_1: "c2_1", uniq_2: "c2_2" }}}
        |      ]
        |    }
        |  }) {
        |    uniq_1
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
        |    uniq_1: "c3_1"
        |    uniq_2: "c3_2"
        |  }) {
        |    uniq_1
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    // Update all As for B1, connecting them to C3
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
        |      updateMany: {
        |        where: { id: { not: 0 }}
        |        data: {
        |          c_id_1: "c3_1"
        |          c_id_2: "c3_2"
        |        }
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

    res.toString() should be("""{"data":{"updateOneModelB":{"a":[{"c":{"uniq_1":"c3_1","uniq_2":"c3_2"}},{"c":{"uniq_1":"c3_1","uniq_2":"c3_2"}}]}}}""")

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
        |      updateMany: {
        |        where: { id: { not: 0 }}
        |        data: {
        |          c_id_1: null
        |        }
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

    res.toString() should be("""{"data":{"updateOneModelB":{"a":[{"c":null},{"c":null}]}}}""")
  }

  "Unchecked nested many updates" should "not allow writing parent inlined relation scalars" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id
         |  b_id Int
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

    // B can't be written because it's the parent.
    server.queryThatMustFail(
      """
        |mutation {
        |  updateOneModelB(where: { id: 1 }, data: {
        |    a: {
        |      updateMany: {
        |        where: { id: 1 }
        |        data: { b_id: 123 }
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

  "Unchecked nested many updates" should "allow to write to autoincrement IDs directly" taggedAs IgnoreMsSql in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id @default(autoincrement())
         |  b_id Int
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
        |    a: { updateMany: { where: { id: { not: 0 }}, data: { id: 111 }}}
        |  }) {
        |    a { id }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateOneModelB":{"a":[{"id":111}]}}}""")
  }
}
