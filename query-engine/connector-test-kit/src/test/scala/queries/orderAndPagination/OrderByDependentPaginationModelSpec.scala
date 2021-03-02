package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class OrderByDependentPaginationModelSpec extends FlatSpec with Matchers with ApiSpecBase {
  implicit val project: Project = SchemaDsl.fromStringV11() {
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
      |  id   Int     @id
      |  b    ModelB?
      |  a_id Int?
      |  a    ModelA? @relation(fields: [a_id], references: [id])
      |}
    """
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
    database.truncateProjectTables(project)
  }

  "[Hops: 1] Ordering by related record field ascending" should "work" taggedAs IgnoreMsSql in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { id: asc }}, cursor: { id: 1 }, take: 2) {
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

  "[Hops: 1] Ordering by related record field descending" should "work" taggedAs IgnoreMsSql in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { id: desc }}, cursor: { id: 4 }, take: 2) {
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

  "[Hops: 1] Ordering by related record field ascending with nulls" should "work" taggedAs IgnoreMsSql in {
    // 1 record has the "full chain", one half, one none
    createRecord(1, Some(1), Some(1))
    createRecord(2, Some(2), None)
    createRecord(3, None, None)

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { id: asc }}, cursor: { id: 1 }, take: 3) {
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

  "[Hops: 2] Ordering by related record field ascending" should "work" taggedAs IgnoreMsSql in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { id: asc }}}, cursor: { id: 1 }, take: 2) {
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

  "[Hops: 2] Ordering by related record field descending" should "work" taggedAs IgnoreMsSql in {
    createRecord(1, Some(2), Some(3))
    createRecord(4, Some(5), Some(6))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { id: desc }}}, cursor: { id: 1 }, take: 2) {
        |    id
        |    b { c { id }}
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString should be("""{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":3}}}]}}""")
  }

  "[Hops: 2] Ordering by related record field ascending with nulls" should "work" taggedAs IgnoreMsSql in {
    // 1 record has the "full chain", one half, one none
    createRecord(1, Some(1), Some(1))
    createRecord(2, Some(2), None)
    createRecord(3, None, None)

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { id: asc }}}, cursor: { id: 1 }, take: 3) {
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
      """{"data":{"findManyModelA":[{"id":2,"b":{"c":null}},{"id":3,"b":null},{"id":1,"b":{"c":{"id":1}}}]}}""",
      """{"data":{"findManyModelA":[{"id":3,"b":null},{"id":2,"b":{"c":null}},{"id":1,"b":{"c":{"id":1}}}]}}""",
      """{"data":{"findManyModelA":[{"id":1,"b":{"c":{"id":1}}},{"id":2,"b":{"c":null}},{"id":3,"b":null}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  "[Circular] Ordering by related record field ascending" should "work" taggedAs IgnoreMsSql in {
    // Records form circles with their relations
    createRecord(1, Some(1), Some(1), Some(1))
    createRecord(2, Some(2), Some(2), Some(2))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { a: { id: asc }}}}, cursor: { id: 1 }, take: 2) {
        |    id
        |    b {
        |      c {
        |        a {
        |          id
        |        }
        |      }
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"findManyModelA":[{"id":1,"b":{"c":{"a":{"id":1}}}},{"id":2,"b":{"c":{"a":{"id":2}}}}]}}""")
  }

  "[Circular] Ordering by related record field descending" should "work" taggedAs IgnoreMsSql in {
    // Records form circles with their relations
    createRecord(1, Some(1), Some(1), Some(1))
    createRecord(2, Some(2), Some(2), Some(2))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { a: { id: desc }}}}, cursor: { id: 1 }, take: 2) {
        |    id
        |    b {
        |      c {
        |        a {
        |          id
        |        }
        |      }
        |    }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"findManyModelA":[{"id":1,"b":{"c":{"a":{"id":1}}}}]}}""")
  }

  "[Circular with differing records] Ordering by related record field ascending" should "work" taggedAs (IgnoreMsSql, IgnoreMySql) in {
    // Records form circles with their relations
    createRecord(1, Some(1), Some(1), Some(3))
    createRecord(2, Some(2), Some(2), Some(4))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { a: { id: asc }}}}, cursor: { id: 1 }, take: 4) {
        |    id
        |    b {
        |      c {
        |        a {
        |          id
        |        }
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
      """{"data":{"findManyModelA":[{"id":3,"b":null},{"id":4,"b":null},{"id":1,"b":{"c":{"a":{"id":3}}}},{"id":2,"b":{"c":{"a":{"id":4}}}}]}}""",
      """{"data":{"findManyModelA":[{"id":1,"b":{"c":{"a":{"id":3}}}},{"id":2,"b":{"c":{"a":{"id":4}}}},{"id":3,"b":null},{"id":4,"b":null}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  "[Circular with differing records] Ordering by related record field descending" should "work" taggedAs (IgnoreMsSql, IgnoreMySql) in {
    // Records form circles with their relations
    createRecord(1, Some(1), Some(1), Some(3))
    createRecord(2, Some(2), Some(2), Some(4))

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: { b: { c: { a: { id: desc }}}}, cursor: { id: 2 }, take: 4) {
        |    id
        |    b {
        |      c {
        |        a {
        |          id
        |        }
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
      """{"data":{"findManyModelA":[{"id":2,"b":{"c":{"a":{"id":4}}}},{"id":1,"b":{"c":{"a":{"id":3}}}},{"id":3,"b":null},{"id":4,"b":null}]}}""",
      """{"data":{"findManyModelA":[{"id":3,"b":null},{"id":4,"b":null},{"id":2,"b":{"c":{"a":{"id":4}}}},{"id":1,"b":{"c":{"a":{"id":3}}}}]}}"""
    )

    possibleResults should contain(result.toString)
  }

  "Multiple relations to the same model and orderBy" should "work" taggedAs IgnoreMsSql in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id    Int     @id
        |
        |  b1_id Int?
        |  b1    ModelB? @relation(fields: [b1_id], references: [id], name: "1")
        |
        |  b2_id Int?
        |  b2    ModelB? @relation(fields: [b2_id], references: [id], name: "2")
        |}
        |
        |model ModelB {
        |  id Int     @id
        |
        |  a1 ModelA[] @relation("1")
        |  a2 ModelA[] @relation("2")
        |}
      """
    }
    database.setup(project)

    server.query(s"""mutation { createOneModelA(data: { id: 1, b1: { create: { id: 1 } }, b2: { create: { id: 10 } } }) { id }}""".stripMargin, project, legacy = false)
    server.query(s"""mutation { createOneModelA(data: { id: 2, b1: { connect: { id: 1 } }, b2: { create: { id: 5 } } }) { id }}""".stripMargin, project, legacy = false)
    server.query(s"""mutation { createOneModelA(data: { id: 3, b1: { create: { id: 2 } }, b2: { create: { id: 7 } } }) { id }}""".stripMargin, project, legacy = false)

    val result = server.query(
      """
        |{
        |  findManyModelA(orderBy: [{ b1: { id: asc } }, { b2: { id: desc } }], cursor: { id: 1 }, take: 3) {
        |    id
        |    b1 { id }
        |    b2 { id }
        |  }
        |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"findManyModelA":[{"id":1,"b1":{"id":1},"b2":{"id":10}},{"id":2,"b1":{"id":1},"b2":{"id":5}},{"id":3,"b1":{"id":2},"b2":{"id":7}}]}}""")
  }

  // Minimal tests specifically for covering the basics in SQL server (no double nulls allowed).
  "Simple orderBy relation" should "work" in {
    implicit val project: Project = SchemaDsl.fromStringV11() {
      """
       |model ModelA {
       |  id   Int     @id
       |  b_id Int?
       |  b    ModelB? @relation(fields: [b_id], references: [id])
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
       |  id   Int     @id
       |  b    ModelB?
       |}
      """
    }
    database.setup(project)

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

  "Simple orderBy relation with two hops" should "work" in {
    implicit val project: Project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id   Int     @id
        |  b_id Int?
        |  b    ModelB? @relation(fields: [b_id], references: [id])
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
        |  id   Int     @id
        |  b    ModelB?
        |}
      """
    }
    database.setup(project)

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



  def createRecord(a_id: Int, b_id: Option[Int], c_id: Option[Int], cToA: Option[Int] = None)(implicit project: Project): Unit = {
    val (followUp, inline) = cToA match {
      case Some(id) if id != a_id => (None, Some(s"a: { create: { id: $id }}"))
      case Some(id)               => (Some(s"mutation { updateOneModelC(where: { id: ${c_id.get} }, data: { a_id: $id }) { id }}"), None)
      case None                   => (None, None)
    }

    val modelC = c_id match {
      case Some(id) => s"""c: { create: { id: $id \n ${inline.getOrElse("")} }}"""
      case None     => ""
    }

    val modelB = b_id match {
      case Some(id) => s"b: { create: { id: $id\n $modelC }}"
      case None     => ""
    }

    val modelA = s"{ id: $a_id \n $modelB }"
    server.query(s"""mutation { createOneModelA(data: $modelA) { id }}""".stripMargin, project, legacy = false)

    followUp match {
      case Some(query) => server.query(query, project, legacy = false)
      case None => ()
    }
  }
}
