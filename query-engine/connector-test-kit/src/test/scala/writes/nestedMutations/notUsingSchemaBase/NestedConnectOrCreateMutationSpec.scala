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

  "A 1!:1! relation connectOrCreate" should "work and prevent relation violations" in {
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
        |  oneA ModelA
        |}
      """.stripMargin
    }
    database.setup(project)

    // Parent inlined cases

    // Both records are new, must succeed
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

    // Updating existing parent, connecting existing child. Fails because the existing child would dangle, but has a required parent.
    server.queryThatMustFail(
      s"""mutation {
         |  updateOneModelA(
         |    where: { id: "A1" }
         |    data: {
         |      oneB: {
         |        connectOrCreate: {
         |          where: { b_u: "B2" }
         |          create: {
         |            id: "B_id_2",
         |            b_u: "B2"
         |          }
         |        }
         |      }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      2014,
      errorContains = "The change you are trying to make would violate the required relation 'ModelAToModelB' between the `ModelA` and `ModelB` models.",
      legacy = false,
    )

    // New parent, connecting existing child. Same as above, fails because the existing child would dangle, but has a required parent.
    server.queryThatMustFail(
      s"""mutation {
         |  updateOneModelA(
         |    where: { id: "A1" }
         |    data: {
         |      oneB: {
         |        connectOrCreate: {
         |          where: { b_u: "B2" }
         |          create: {
         |            id: "B_id_2",
         |            b_u: "B2"
         |          }
         |        }
         |      }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      2014,
      errorContains = "The change you are trying to make would violate the required relation 'ModelAToModelB' between the `ModelA` and `ModelB` models.",
      legacy = false,
    )

    // Child inlined cases
    result = server.query(
      s"""mutation {
         |  createOneModelB(data: {
         |    id: "B_id_2"
         |    b_u: "B2"
         |    oneA: {
         |      connectOrCreate: {
         |        where: { id: "A2" }
         |        create: {
         |          id: "A2",
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    oneA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelB\":{\"id\":\"B_id_2\",\"oneA\":{\"id\":\"A2\"}}}}")

    // Updating existing parent (B), connecting existing child (A). Fails because the existing child would dangle, but has a required parent.
    server.queryThatMustFail(
      s"""mutation {
         |  updateOneModelB(
         |    where: { b_u: "B2" }
         |    data: {
         |      oneA: {
         |        connectOrCreate: {
         |          where: { id: "A2" }
         |          create: {
         |            id: "A2",
         |          }
         |        }
         |      }
         |  }) {
         |    id
         |    oneA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      2014,
      errorContains = "The change you are trying to make would violate the required relation 'ModelAToModelB' between the `ModelA` and `ModelB` models.",
      legacy = false,
    )

    // New parent, connecting existing child. Same as above, fails because the existing child would dangle, but has a required parent.
    server.queryThatMustFail(
      s"""mutation {
         |  createOneModelB(
         |    data: {
         |      id: "B_id_3"
         |      b_u: "B3"
         |      oneA: {
         |        connectOrCreate: {
         |          where: { id: "A2" }
         |          create: {
         |            id: "A2"
         |          }
         |        }
         |      }
         |  }) {
         |    id
         |    oneA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      2014,
      errorContains = "The change you are trying to make would violate the required relation 'ModelAToModelB' between the `ModelA` and `ModelB` models.",
      legacy = false,
    )
  }

  "A 1:1! relation connectOrCreate" should "work and prevent relation violations" in {
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
        |  oneA ModelA
        |}
      """.stripMargin
    }
    database.setup(project)

    // Parent inlined cases

    // Both records are new, must succeed
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

    // Updating existing parent, connecting existing child. Fails because the existing child would dangle, but has a required parent.
    server.queryThatMustFail(
      s"""mutation {
         |  updateOneModelA(
         |    where: { id: "A1" }
         |    data: {
         |      oneB: {
         |        connectOrCreate: {
         |          where: { b_u: "B2" }
         |          create: {
         |            id: "B_id_2",
         |            b_u: "B2"
         |          }
         |        }
         |      }
         |  }) {
         |    id
         |    oneB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      2014,
      errorContains = "The change you are trying to make would violate the required relation 'ModelAToModelB' between the `ModelA` and `ModelB` models.",
      legacy = false,
    )

    // New parent, connecting existing child. Existing parent has no child.
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

    result.toString() should be("{\"data\":{\"findOneModelA\":{\"oneB\":null}}}")

    // Child inlined cases

    // New Parent (B), new child (A)
    result = server.query(
      s"""mutation {
         |  createOneModelB(data: {
         |    id: "B_id_2"
         |    b_u: "B2"
         |    oneA: {
         |      connectOrCreate: {
         |        where: { id: "A3" }
         |        create: {
         |          id: "A3",
         |        }
         |      }
         |    }
         |  }) {
         |    id
         |    oneA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("{\"data\":{\"createOneModelB\":{\"id\":\"B_id_2\",\"oneA\":{\"id\":\"A3\"}}}}")

    // Updating existing parent (B), connecting existing child (A). Fails because the existing child would dangle, but has a required parent.
    server.queryThatMustFail(
      s"""mutation {
         |  updateOneModelB(
         |    where: { b_u: "B2" }
         |    data: {
         |      oneA: {
         |        connectOrCreate: {
         |          where: { id: "A2" }
         |          create: {
         |            id: "A2",
         |          }
         |        }
         |      }
         |  }) {
         |    id
         |    oneA {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      2014,
      errorContains = "The change you are trying to make would violate the required relation 'ModelAToModelB' between the `ModelA` and `ModelB` models.",
      legacy = false,
    )
  }
}
