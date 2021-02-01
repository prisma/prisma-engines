package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class OrderByDependentModelSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """
      |model ModelA {
      |  id   Int     @id
      |  b_id Int?
      |  b    ModelB? @relation(fields: [b_id], references: [id])
      |  c    ModelC?
      |}
      |
      |model ModelB {
      |  id Int     @id
      |  a  ModelA?
      |
      |  c_id Int?
      |  c    ModelC? @relation(fields: [c_id], references: [id])
      |}
      |
      |model ModelC {
      |  id Int @id
      |  b ModelB?
      |}
    """
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
    database.truncateProjectTables(project)
  }

  "[Hops: 1] Ordering by related record field ascending" should "work" in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { id: asc }}) {
        |    id
        |    b {
        |      id
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyModelA":[{"id":1,"b":{"id":2}},{"id":4,"b":{"id":5}}]}}""")
  }

  "[Hops: 1] Ordering by related record field descending" should "work" in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { id: desc }}) {
        |    id
        |    b {
        |      id
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyModelA":[{"id":4,"b":{"id":5}},{"id":1,"b":{"id":2}}]}}""")
  }

  "[Hops: 1] Ordering by related record field ascending with nulls" should "work" in {
    // 1 record has the "full chain", one half, one none
    createRecord(1, Some(1), Some(1))
    createRecord(2, Some(2), None)
    createRecord(3, None, None)

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { id: asc }}) {
        |    id
        |    b {
        |      id
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    // Depends on how null values are handled.
    val possibleResults = Seq(
      """{"data":{"findManyModelA":[{"id":3,"b":null},{"id":1,"b":{"id":1}},{"id":2,"b":{"id":2}}]}}""",
      """{"data":{"findManyModelA":[{"id":1,"b":{"id":1}},{"id":2,"b":{"id":2}},{"id":3,"b":null}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  "[Hops: 2] Ordering by related record field ascending" should "work" in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { id: asc }}}) {
        |    id
        |    b { c { id }}
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":3}}},{"id":4,"b":{"c":{"id":6}}}]}}""")
  }

  "[Hops: 2] Ordering by related record field descending" should "work" in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { id: desc }}}) {
        |    id
        |    b { c { id }}
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyModelA":[{"id":4,"b":{"c":{"id":6}}},{"id":1,"b":{"c":{"id":3}}}]}}""")
  }

  "[Hops: 2] Ordering by related record field ascending with nulls" should "work" in {
    // 1 record has the "full chain", one half, one none
    createRecord(1, Some(1), Some(1))
    createRecord(2, Some(2), None)
    createRecord(3, None, None)

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { id: asc }}}) {
        |    id
        |    b {
        |      c {
        |        id
        |      }
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    // Depends on how null values are handled.
    val possibleResults = Seq(
      """{"data":{"findManyModelA":[{"id":3,"b":null},{"id":2,"b":{"c":null}},{"id":1,"b":{"c":{"id":1}}}]}}""",
      """{"data":{"findManyModelA":[{"id":3,"b":null},{"id":2,"b":{"c":null}},{"id":1,"b":{"c":{"id":1}}}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  def createRecord(a_id: Int, b_id: Option[Int], c_id: Option[Int]): Unit = {
    val modelC = c_id match {
      case Some(id) => s"c: { create: { id: $id }}"
      case None     => ""
    }

    val modelB = b_id match {
      case Some(id) => s"b: { create: { id: $id\n $modelC }}"
      case None     => ""
    }

    val modelA = s"{ id: $a_id \n $modelB }"
    server.query(s"""mutation { createOneModelA(data: $modelA) { id }}""".stripMargin, project, legacy = false)
  }
}
