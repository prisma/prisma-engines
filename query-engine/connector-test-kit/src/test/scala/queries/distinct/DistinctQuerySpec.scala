package queries.distinct

import org.scalatest.{FlatSpec, Matchers}
import util._

class DistinctQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """model ModelA {
      |  id     String @id @default(cuid())
      |  fieldA String
      |  fieldB Int
      |
      |  b ModelB[]
      |}
      |
      |model ModelB {
      |  id    String @id @default(cuid())
      |  field String
      |  a_id  String
      |  a     ModelA @relation(fields: [a_id], references: [id])
      |}
    """.stripMargin
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
  }

  def createRecord(fieldA: String, fieldB: Int, nested: Option[Seq[String]] = None) = {
    val nested_query = nested match {
      case Some(list) =>
        val creates = list.map(field => s"""{ field: "$field" }""").mkString(",")
        s"""b: { create: [ $creates ] }"""

      case None => ""
    }

    server.query(
      s"""mutation {
         |  createOneModelA(data: { fieldA: "$fieldA", fieldB: $fieldB, $nested_query }) {
         |    id
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )
  }

  "Select distinct with no records in the database" should "return nothing" in {
    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB]) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyModelA":[]}}""")
  }

  "Select distinct with a duplicate in the database" should "return only distinct records" in {
    createRecord("1", 1)
    createRecord("2", 2)
    createRecord("1", 1)

    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB]) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyModelA":[{"fieldA":"1","fieldB":1},{"fieldA":"2","fieldB":2}]}}""")
  }

  "Select distinct with skip" should "return only distinct records after the skip" in {
    createRecord("1", 1)
    createRecord("2", 2)
    createRecord("1", 1)

    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB], skip: 1) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyModelA":[{"fieldA":"2","fieldB":2}]}}""")
  }

  "Select distinct with skip and ordering" should "return only distinct records after the skip, ordered correctly" in {
    createRecord("1", 1)
    createRecord("2", 2)
    createRecord("1", 1)

    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB], skip: 1, orderBy: { fieldB: DESC }) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"findManyModelA":[{"fieldA":"1","fieldB":1}]}}""")
  }

  "Select distinct with ordering on a non-distinct-by field" should "return only distinct records, ordered correctly" in {
    // CUIDs are linear ordered in time
    createRecord("1", 1) // Lowest ID
    createRecord("2", 2)
    createRecord("1", 1)
    createRecord("3", 1) // Highest ID

    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB], orderBy: { id: DESC }) {
         |    fieldA
         |    fieldB
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // 3, 1
    // 1, 1
    // 2, 2
    result.toString() should be("""{"data":{"findManyModelA":[{"fieldA":"3","fieldB":1},{"fieldA":"1","fieldB":1},{"fieldA":"2","fieldB":2}]}}""")
  }

  // todo change to comparable ids
  "Select distinct on top level and relation" should "return only distinct records for top record, and only for those the distinct relation records" in {
    createRecord("1", 1, Some(Seq("3", "1", "1", "2", "1"))) // Lowest ID (nested: lowest first, highest last)
    createRecord("1", 1, Some(Seq("1", "2")))
    createRecord("1", 3, None)
    createRecord("1", 4, Some(Seq("1", "1")))
    createRecord("1", 5, Some(Seq("2", "3", "2"))) // Highest ID (nested: lowest first, highest last)

    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB]) {
         |    fieldA
         |    fieldB
         |    b(distinct: [field], orderBy: { id: ASC }) {
         |      field
         |    }
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // 1, 1 => 3, 1, 2
    // 1, 3 => -
    // 1, 4 => 1
    // 1, 5 => 2, 3
    result.toString() should be(
      """{"data":{"findManyModelA":[{"fieldA":"1","fieldB":1,"b":[{"field":"3"},{"field":"1"},{"field":"2"}]},{"fieldA":"1","fieldB":3,"b":[]},{"fieldA":"1","fieldB":4,"b":[{"field":"1"}]},{"fieldA":"1","fieldB":5,"b":[{"field":"2"},{"field":"3"}]}]}}""")
  }

  "Select distinct on top level and relation, ordering reversed" should "return only distinct records for top record, and only for those the distinct relation records with correct ordering" in {
    createRecord("1", 1, Some(Seq("3", "1", "1", "2", "1")))
    createRecord("1", 1, Some(Seq("1", "2")))
    createRecord("1", 3, None)
    createRecord("1", 4, Some(Seq("1", "1")))
    createRecord("1", 5, Some(Seq("2", "3", "2")))

    val result = server.query(
      s"""{
         |  findManyModelA(distinct: [fieldA, fieldB], orderBy: { fieldB: DESC}) {
         |    fieldA
         |    fieldB
         |    b(distinct: [field], orderBy: { id: DESC }) {
         |      field
         |    }
         |  }
         |}""".stripMargin,
      project,
      legacy = false
    )

    // 1, 5 => 2, 3
    // 1, 4 => 1
    // 1, 3 => -
    // 1, 1 => 1, 2, 3
    result.toString() should be(
      """{"data":{"findManyModelA":[{"fieldA":"1","fieldB":5,"b":[{"field":"2"},{"field":"3"}]},{"fieldA":"1","fieldB":4,"b":[{"field":"1"}]},{"fieldA":"1","fieldB":3,"b":[]},{"fieldA":"1","fieldB":1,"b":[{"field":"1"},{"field":"2"},{"field":"3"}]}]}}""")
  }
}
