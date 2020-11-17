package queries.batch

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, IgnoreMsSql, IgnoreMySql, IgnorePostgres, IgnoreSQLite, ProjectDsl}

class TransactionalBatchSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = ProjectDsl.fromString {
    """
      |model ModelA {
      |  id   Int @id
      |  b_id Int?
      |  b ModelB? @relation(fields: [b_id], references: [id])
      |}
      |
      |model ModelB {
      |  id Int @id
      |  a  ModelA?
      |}
      |
      |model ModelC {
      |  id Int @id
      |}
      """
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.truncateProjectTables(project)
  }

  "A transactional batch of successful queries" should "work" taggedAs (IgnoreMsSql) in {
    val queries = Seq(
      """mutation { createOneModelA(data: { id: 1 }) { id }}""",
      """mutation { createOneModelA(data: { id: 2 }) { id }}""",
    )

    server.batch(queries, transaction = true, project, legacy = false).toString should be(
      """[{"data":{"createOneModelA":{"id":1}}},{"data":{"createOneModelA":{"id":2}}}]"""
    )
  }

  "A transactional batch with one failing query" should "roll back all changes" in {
    val queries = Seq(
      """mutation { createOneModelA(data: { id: 1 }) { id }}""",
      """mutation { createOneModelA(data: { id: 1 }) { id }}""",
    )

    server.batch(queries, transaction = true, project, legacy = false).toString should startWith(
      """{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: Some(KnownError { message: \"Unique constraint failed"""
    )

    val result = server.query("""
        |{
        |  findManyModelA {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false)

    result.toString() should be("""{"data":{"findManyModelA":[]}}""")
  }

  "A single-query batch with a write query" should "be transactional in itself (roll back all changes)" in {
    // Existing ModelA in the DB will prevent the nested ModelA creation in the batch.
    server.query("""
      |mutation {
      |  createOneModelA(data: { id: 1 }) {
      |    id
      |  }
      |}
    """, project, legacy = false)

    val queries = Seq(
      """mutation { createOneModelB(data: { id: 1, a: { create: { id: 1 } } }) { id }}""", // ModelB gets created before ModelA because of inlining
    )

    server.batch(queries, transaction = false, project, legacy = false).toString should startWith(
      """[{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: Some(KnownError { message: \"Unique constraint failed"""
    )

    val result = server.query("""
        |{
        |  findManyModelB {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false)

    result.toString() should be("""{"data":{"findManyModelB":[]}}""")
  }

  // Postgres only for basic testing.
  "A batch with a raw and regular queries mixed" should "be able to execute" taggedAs(IgnoreMySql, IgnoreSQLite, IgnoreMsSql) in {
    server.query("""
                   |mutation {
                   |  createOneModelA(data: { id: 1 }) {
                   |    id
                   |  }
                   |}
                 """, project, legacy = false)

    val queries = Seq(
      """mutation { createOneModelB(data: { id: 1, a: { connect: { id: 1 } } }) { id }}""",
      """mutation { executeRaw(query: "INSERT INTO \"ModelA\" (id, b_id) VALUES(2, NULL)", parameters: "[]") }""",
      """mutation { queryRaw(query: "SELECT * FROM \"ModelC\"", parameters: "[]") }"""
    )

    server.batch(queries, transaction = true, project, legacy = false).toString should be("""[{"data":{"createOneModelB":{"id":1}}},{"data":{"executeRaw":1}},{"data":{"queryRaw":[]}}]""")
  }
}
