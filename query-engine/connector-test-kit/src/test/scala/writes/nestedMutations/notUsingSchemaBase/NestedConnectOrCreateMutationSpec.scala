package writes.nestedMutations.notUsingSchemaBase

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

// Note: Except for m:n cases that are always resolved using the primary identifier of the models, we use different
// relation links to ensure that the underlying QE logic correctly uses link resolvers instead of
// only primary id resolvers.
class NestedConnectOrCreateMutationSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "A m:n relation connectOrCreate" should "always work" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id    String   @id @default(cuid())
        |  manyB ModelB[]
        |}
        |
        |model ModelB {
        |  id    String   @id @default(cuid())
        |  manyA ModelA[]
        |}
      """.stripMargin
    }
    database.setup(project)

    // Both records are new
    var result = server.query(
      s"""mutation{
           |  createOneModelA(data: {
           |    id: "A1"
           |    manyB: {
           |      connectOrCreate: {
           |        where: { id: "B1" }
           |        create: {
           |          id: "B1"
           |        }
           |      }
           |    }
           |  }) {
           |    id
           |    manyB {
           |      id
           |    }
           |  }
           |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelA\":{\"id\":\"A1\",\"manyB\":[{\"id\":\"B1\"}]}}}")

    // New parent, connect existing child
    result = server.query(
      s"""mutation{
         |  createOneModelA(data: {
         |    id: "A2"
         |    manyB: {
         |      connectOrCreate: {
         |        where: { id: "B1" }
         |        create: {
         |          id: "Doesn't matter"
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelA\":{\"id\":\"A2\",\"manyB\":[{\"id\":\"B1\"}]}}}")

    // Update a parent to connect 2 new children
    result = server.query(
      s"""mutation {
         |  updateOneModelA(
         |    where: { id: "A1" }
         |    data: {
         |      manyB: {
         |        connectOrCreate: [{
         |          where: { id: "B2" }
         |          create: {
         |            id: "B2"
         |          }
         |        },{
         |          where: { id: "B3" }
         |          create: {
         |            id: "B3"
         |          }
         |        }]
         |      }
         |    }
         |  ) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"updateOneModelA\":{\"id\":\"A1\",\"manyB\":[{\"id\":\"B1\"},{\"id\":\"B2\"},{\"id\":\"B3\"}]}}}")
  }

  "A 1!:m relation connectOrCreate" should "work" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id  String @id @default(cuid())
        |  b_u String
        |
        |  oneB ModelB @relation(fields: [b_u], references: [b_u])
        |}
        |
        |model ModelB {
        |  id  String @id @default(cuid())
        |  b_u String @unique
        |
        |  manyA ModelA[]
        |}
      """.stripMargin
    }
    database.setup(project)

    // Inlined in parent cases

    // Both records are new
    var result = server.query(
      s"""mutation {
         |  createOneModelA(data: {
         |    id: "A1"
         |    oneB: {
         |      connectOrCreate: {
         |        where: { b_u: "B1" }
         |        create: {
         |          id: "B_id_1",
         |          b_u: "B1"
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelA\":{\"id\":\"A1\",\"oneB\":{\"id\":\"B_id_1\"}}}}")

    // Create new parent, connect to existing child
    result = server.query(
      s"""mutation {
         |  createOneModelA(data: {
         |    id: "A2"
         |    oneB: {
         |      connectOrCreate: {
         |        where: { b_u: "B1" }
         |        create: {
         |          id: "B_id_1",
         |          b_u: "B1"
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelA\":{\"id\":\"A2\",\"oneB\":{\"id\":\"B_id_1\"}}}}")

    // Inlined in child cases

    // Connect 2 more children (ModelAs here)
    result = server.query(
      s"""mutation {
         |  updateOneModelB(
         |    where: { b_u: "B1" }
         |    data: {
         |      manyA: {
         |        connectOrCreate: [{
         |          where: { id: "A3" }
         |          create: {
         |            id: "A3"
         |          }
         |        },{
         |          where: { id: "A4" }
         |          create: {
         |            id: "A4"
         |          }
         |        }]
         |      }
         |    }
         |  ) {
         |    id
         |    manyA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be(
      "{\"data\":{\"updateOneModelB\":{\"id\":\"B_id_1\",\"manyA\":[{\"id\":\"A1\"},{\"id\":\"A2\"},{\"id\":\"A3\"},{\"id\":\"A4\"}]}}}")

    // Create new child, connect existing parent (disconnects parent from B1)
    result = server.query(
      s"""mutation {
         |  createOneModelB(
         |    data: {
         |      id: "B_id_2"
         |      b_u: "B2",
         |      manyA: {
         |        connectOrCreate: {
         |          where: { id: "A1" }
         |          create: {
         |            id: "A1"
         |          }
         |        }
         |      }
         |    }
         |  ) {
         |    id
         |    manyA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelB\":{\"id\":\"B_id_2\",\"manyA\":[{\"id\":\"A1\"}]}}}")

    result = server.query(
      s"""{
         |  findOneModelA(where: { id: "A1" }) {
         |    oneB {
         |      b_u
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"findOneModelA\":{\"oneB\":{\"b_u\":\"B2\"}}}}")
  }

  "A 1:m relation connectOrCreate" should "work" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id  String @id @default(cuid())
        |  b_u String?
        |
        |  oneB ModelB? @relation(fields: [b_u], references: [b_u])
        |}
        |
        |model ModelB {
        |  id  String @id @default(cuid())
        |  b_u String @unique
        |
        |  manyA ModelA[]
        |}
      """.stripMargin
    }
    database.setup(project)

    // Inlined in parent cases

    // Both records are new
    var result = server.query(
      s"""mutation {
         |  createOneModelA(data: {
         |    id: "A1"
         |    oneB: {
         |      connectOrCreate: {
         |        where: { b_u: "B1" }
         |        create: {
         |          id: "B_id_1",
         |          b_u: "B1"
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelA\":{\"id\":\"A1\",\"oneB\":{\"id\":\"B_id_1\"}}}}")

    // Create new parent, connect to existing child
    result = server.query(
      s"""mutation {
         |  createOneModelA(data: {
         |    id: "A2"
         |    oneB: {
         |      connectOrCreate: {
         |        where: { b_u: "B1" }
         |        create: {
         |          id: "B_id_1",
         |          b_u: "B1"
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelA\":{\"id\":\"A2\",\"oneB\":{\"id\":\"B_id_1\"}}}}")

    // Inlined in child cases

    // Connect 2 more children (ModelAs here)
    result = server.query(
      s"""mutation {
         |  updateOneModelB(
         |    where: { b_u: "B1" }
         |    data: {
         |      manyA: {
         |        connectOrCreate: [{
         |          where: { id: "A3" }
         |          create: {
         |            id: "A3"
         |          }
         |        },{
         |          where: { id: "A4" }
         |          create: {
         |            id: "A4"
         |          }
         |        }]
         |      }
         |    }
         |  ) {
         |    id
         |    manyA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be(
      "{\"data\":{\"updateOneModelB\":{\"id\":\"B_id_1\",\"manyA\":[{\"id\":\"A1\"},{\"id\":\"A2\"},{\"id\":\"A3\"},{\"id\":\"A4\"}]}}}")

    // Create new child, connect existing parent (disconnects parent from B1)
    result = server.query(
      s"""mutation {
         |  createOneModelB(
         |    data: {
         |      id: "B_id_2"
         |      b_u: "B2",
         |      manyA: {
         |        connectOrCreate: {
         |          where: { id: "A1" }
         |          create: {
         |            id: "A1"
         |          }
         |        }
         |      }
         |    }
         |  ) {
         |    id
         |    manyA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelB\":{\"id\":\"B_id_2\",\"manyA\":[{\"id\":\"A1\"}]}}}")

    result = server.query(
      s"""{
         |  findOneModelA(where: { id: "A1" }) {
         |    oneB {
         |      b_u
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"findOneModelA\":{\"oneB\":{\"b_u\":\"B2\"}}}}")
  }

  // Regression test for failing internal graph transformations.
  "Query reordering" should "not break connectOrCreate" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model A {
        |  id     String  @id
        |  fieldA String?
        |  A2B    A2B[]   @relation("A2_A2B")
        |}
        |
        |model B {
        |  id     String @id
        |  fieldB String
        |  A2B    A2B[]  @relation("B2_A2B")
        |}
        |
        |model A2B {
        |  a_id    String
        |  b_id    String
        |  fieldAB Int
        |  a       A      @relation("A2_A2B", fields: [a_id], references: [id])
        |  b       B      @relation("B2_A2B", fields: [b_id], references: [id])
        |
        |  @@id([a_id, b_id])
        |  @@index([b_id], name: "fk_b")
        |  @@map("_A2B")
        |}
      """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      s"""mutation {upsertOneA2B(
         |  where: {
         |    a_id_b_id: {
         |      a_id: "a"
         |      b_id: "b"
         |    }
         |  },
         |  create: {
         |    a: {
         |      connectOrCreate: {
         |        where:  { id: "a" }
         |        create: { id: "a", fieldA: "Field A" }
         |      }
         |    }
         |    b: {
         |      connectOrCreate: {
         |        where:  { id: "b" }
         |        create: { id: "b", fieldB: "Field B" }
         |      }
         |    }
         |    fieldAB: 1
         |  }
         |  update: {
         |    fieldAB: 1
         |  }) {
         |    fieldAB
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"upsertOneA2B":{"fieldAB":1}}}""")
  }
}
