package writes.unchecked_writes

import org.scalatest.{FlatSpec, Matchers}
import util._

// Important: This test covers ALL top level update inputs, like update & upsert,
// because schema building uses the exact same types under the hood.
class UncheckedUpdateSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Unchecked updates" should "allow writing inlined relation scalars" in {
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

    // New B & C to connect to.
    server.query(
      """
        |mutation {
        |  createOneModelB(data: {
        |    uniq_1: "b2_1"
        |    uniq_2: "b2_2"
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
        |    uniq_1: "c2_1"
        |    uniq_2: "c2_2"
        |  }) {
        |    uniq_1
        |    uniq_2
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    // Update inlined
    var res = server.query(
      """
        |mutation {
        |  updateOneModelA(where: { id: 1 }, data: {
        |    b_id_1: "b2_1"
        |    b_id_2: "b2_2"
        |    c_id_1: "c2_1"
        |    c_id_2: "c2_2"
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

    res.toString() should be("""{"data":{"updateOneModelA":{"id":1,"b":{"uniq_1":"b2_1","uniq_2":"b2_2"},"c":{"uniq_1":"c2_1","uniq_2":"c2_2"}}}}""")

    res = server.query(
      """
        |mutation {
        |  updateOneModelA(where: { id: 1 }, data: {
        |    c_id_1: null
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

    res.toString() should be("""{"data":{"updateOneModelA":{"id":1,"b":{"uniq_1":"b2_1","uniq_2":"b2_2"},"c":null}}}""")
  }

  "Unchecked updates" should "not allow writing inlined relations regularly" in {
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

    // Update inlined
    server.queryThatMustFail(
      """
        |mutation {
        |  updateOneModelA(where: { id: 1 }, data: {
        |    id: 1
        |    b_id_1: "b2_1"
        |    b_id_2: "b2_2"
        |    c: { create: { uniq_1: "c2_1", uniq_2: "c2_2" } }
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

  "Unchecked updates" should "allow writing non-inlined relations normally" in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int     @id
         |  b_id Int?
         |  b    ModelB? @relation(fields: [b_id], references: [id])
         |  c    ModelC?
         |}
         |
         |model ModelB {
         |  id Int    @id
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

    // Setup
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

    server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1
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

    // Update inline: B is inlined, c is not.
    val res = server.query(
      """
        |mutation {
        |  updateOneModelA(where: { id: 1 }, data: {
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
  }

  "Unchecked updates" should "allow to write to autoincrement IDs directly" taggedAs IgnoreMsSql in {
    val project = ProjectDsl.fromString {
      """|model ModelA {
         |  id   Int    @id @default(autoincrement())
         |}
      """
    }

    database.setup(project)

    val id = server
      .query(
        """
        |mutation {
        |  createOneModelA {
        |    id
        |  }
        |}
      """.stripMargin,
        project,
        legacy = false
      )
      .pathAsInt("data.createOneModelA.id")

    val res = server.query(
      s"""
        |mutation {
        |  updateOneModelA(where: { id: $id }, data: { id: 111 }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    res.toString() should be("""{"data":{"updateOneModelA":{"id":111}}}""")
  }
}
