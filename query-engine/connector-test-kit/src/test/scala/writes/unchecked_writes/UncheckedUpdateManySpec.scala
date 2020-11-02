package writes.unchecked_writes

import org.scalatest.{FlatSpec, Matchers}
import util._

class UncheckedUpdateManySpec extends FlatSpec with Matchers with ApiSpecBase {
  "Unchecked update many" should "allow writing inlined relation scalars" in {
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
        |  createOneModelA(data: {
        |    id: 2
        |    b: { create: { uniq_1: "b2_1", uniq_2: "b2_2" }}
        |    c: { create: { uniq_1: "c2_1", uniq_2: "c2_2" }}
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    // Connect all As to b2 and c2
    var res = server.query(
      """
        |mutation {
        |  updateManyModelA(where: { id: { not: 0 } }, data: {
        |    b_id_1: "b2_1"
        |    b_id_2: "b2_2"
        |    c_id_1: "c2_1"
        |    c_id_2: "c2_2"
        |  }) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateManyModelA":{"count":2}}}""")

    res = server.query(
      """
        |mutation {
        |  updateManyModelA(where: { id: { not: 0 }}, data: {
        |    c_id_1: null
        |  }) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateManyModelA":{"count":2}}}""")
  }

  "Unchecked updates" should "allow to write to autoincrement IDs directly" taggedAs (IgnoreMsSql, IgnoreSQLite) in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id  Int @id
         |  int Int @default(autoincrement())
         |
         |  @@index([int])
         |}
      """
    }

    database.setup(project)

    server
      .query(
        """
        |mutation {
        |  createOneModelA(data: { id: 1 }) {
        |    id
        |  }
        |}
      """.stripMargin,
        project,
        legacy = false
      )

    server
      .query(
        """
          |mutation {
          |  createOneModelA(data: { id: 2 }) {
          |    id
          |  }
          |}
        """.stripMargin,
        project,
        legacy = false
      )

    val res = server.query(
      s"""
        |mutation {
        |  updateManyModelA(where: { id: { not: 0 }}, data: { int: 111 }) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateManyModelA":{"count":2}}}""")
  }
}
