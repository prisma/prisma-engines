package queries

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class RelatedNullQueries extends FlatSpec with Matchers with ApiSpecBase {
  "Querying a single-field 1:n relation with nulls" should "ignore related records connected with null" in {
    val project = ProjectDsl.fromString {
      s"""
         |model ModelA {
         |  id String   @id @default(uuid())
         |  u  String?  @unique
         |  bs ModelB[]
         |}
         |
         |model ModelB {
         |  id  String  @id @default(uuid())
         |  a_u String?
         |  a   ModelA? @relation(fields: [a_u], references: [u])
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createModelA(data: { id: "1", bs: { create: {} } }){
        |    id
        |    bs {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be("{\"data\":{\"createModelA\":{\"id\":\"1\",\"bs\":[]}}}")
  }

  "Querying a multi-field 1:n relation with nulls" should "ignore related records connected with any null in the relation fields" in {
    val project = ProjectDsl.fromString {
      s"""
         |model ModelA {
         |  id String   @id @default(uuid())
         |  u1 String?
         |  u2 String?
         |  bs ModelB[]
         |
         |  @@unique([u1, u2])
         |}
         |
         |model ModelB {
         |  id   String  @id @default(uuid())
         |  a_u1 String?
         |  a_u2 String?
         |  a    ModelA? @relation(fields: [a_u1, a_u2], references: [u1, u2])
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createModelA(data: { id: "1", bs: { create: { } } }){
        |    id
        |    bs {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be("{\"data\":{\"createModelA\":{\"id\":\"1\",\"bs\":[]}}}")

    val result2 = server.query(
      """
        |mutation {
        |  createModelA(data: { id: "2", u1: "u1", bs: { create: { } } }){
        |    id
        |    bs {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result2.toString should be("{\"data\":{\"createModelA\":{\"id\":\"2\",\"bs\":[]}}}")
  }

  "Querying a single-field 1:1 relation inlined on the child with null" should "not find a related record" in {
    val project = ProjectDsl.fromString {
      s"""
         |model ModelA {
         |  id String  @id @default(uuid())
         |  u  String? @unique
         |  b  ModelB?
         |}
         |
         |model ModelB {
         |  id  String  @id @default(uuid())
         |  a_u String?
         |  a   ModelA? @relation(fields: [a_u], references: [u])
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createModelA(data: { id: "1", b: { create: { } } }){
        |    id
        |    b {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be("{\"data\":{\"createModelA\":{\"id\":\"1\",\"b\":null}}}")
  }

  "Querying a single-field 1:1 relation inlined on the parent with null" should "not find a related record" in {
    val project = ProjectDsl.fromString {
      s"""
         |model ModelA {
         |  id  String  @id @default(uuid())
         |  b_u String?
         |  b   ModelB? @relation(fields: [b_u], references: [u])
         |}
         |
         |model ModelB {
         |  id String  @id @default(uuid())
         |  u  String? @unique
         |  a  ModelA?
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createModelA(data: { id: "1", b: { create: { } } }){
        |    id
        |    b {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be("{\"data\":{\"createModelA\":{\"id\":\"1\",\"b\":null}}}")
  }
}
