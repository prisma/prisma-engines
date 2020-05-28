package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import util._

class PaginationSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = SchemaDsl.fromStringV11() {
    """
      |model TestModel {
      |  id          Int    @id
      |  field       String
      |  uniqueField String @unique
      |}
    """.stripMargin
  }

  private def createTestData(): Unit = {
    for (i <- 1 to 10) {
      server.query(s"""mutation { createOneTestModel(data: { id: $i, field: "Field${Math.max(i - 1 + (i % 2), 0)}", uniqueField: "Unique$i" }) { id } }""",
                   project,
                   legacy = false)
    }
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    createTestData()
  }

  /**
    * Cursor only tests.
    */
  "A cursor (on ID) query" should "return all records after and including the cursor" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}""")
  }

  "A cursor (on ID) query with an ordering" should "return all records after and including the cursor" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, orderBy: id_DESC) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}""")
  }

  "A cursor (on ID) query with an ordering on a non-unique field" should "return all records after and including the cursor" in {
    // This test checks that the result is implicitly ordered by ID ASC to guarantee a stable ordering of results, because a non-unique field
    // can't guarantee a stable ordering in itself.
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, orderBy: field_DESC) {
          |    id
          |    field
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be(
      """{"data":{"findManyTestModel":[{"id":5,"field":"Field5"},{"id":6,"field":"Field5"},{"id":3,"field":"Field3"},{"id":4,"field":"Field3"},{"id":1,"field":"Field1"},{"id":2,"field":"Field1"}]}}""")
  }

  "A cursor (on ID) on the end of records" should "return only the last record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 10
          |  }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":10}]}}""")
  }

  "A cursor (on ID) on the first record but with reversed order" should "return only the first record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 1
          |  }, orderBy: id_DESC) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1}]}}""")
  }

  "A cursor (on ID) on a non-existant cursor" should "return no records" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 999
          |  }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  "A cursor (on a unique)" should "work as well" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    uniqueField: "Unique5"
          |  }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}""")
  }

  /**
    * Take only tests.
    */
  "Taking 1" should "return only the first record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(take: 1) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1}]}}""")
  }

  "Taking 1 with reversed order" should "return only the last record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(take: 1, orderBy: id_DESC) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":10}]}}""")
  }

  "Taking 0" should "return no records" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(take: 0) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  "Taking -1 without a cursor" should "return the last record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(take: -1) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":10}]}}""")
  }

  /**
    * Skip only tests.
    */
  "A skip" should "return all records after the offset specified" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(skip: 5) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}""")
  }

  "A skip with order reversed" should "return all records after the offset specified" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(skip: 5, orderBy: id_DESC) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}""")
  }

  "A skipping beyond all records" should "return no records" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(skip: 999) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  /**
    * Cursor + Take tests.
    */
  "A cursor with take 2" should "return the cursor plus one record after the cursor" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: 2) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}""")
  }

  "A cursor with take -2" should "return the cursor plus one record before the cursor" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: -2) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":4},{"id":5}]}}""")
  }

  "A cursor on the last record with take 2" should "return only the cursor record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 10
          |  }, take: 2) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":10}]}}""")
  }

  "A cursor on the first record with take -2" should "return only the cursor record" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 1
          |  }, take: -2) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1}]}}""")
  }

  "A cursor with take 0" should "return no records" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 1
          |  }, take: 0) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  "A cursor with take 2 and reversed ordering" should "return the cursor record and the one before (in the original ordering)" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: 2, orderBy: id_DESC) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":4}]}}""")
  }

  "A cursor with take -2 and reversed ordering" should "return the cursor record and the one after (in the original ordering)" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: -2, orderBy: id_DESC) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}""")
  }

  /**
  * Cursor + Take + Skip tests.
  */
}
