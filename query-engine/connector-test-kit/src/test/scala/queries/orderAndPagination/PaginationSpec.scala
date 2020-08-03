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

  /*
   * Creates 10 test data records with IDs 1 - 10, and 2 adjacent records share the value of the non-unique field.
   */
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

  /***********************
    * Cursor only tests. *
    **********************/
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
          |  }, orderBy: { id: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}""")
  }

  "A cursor (on ID) query with a descending order on a non-unique field" should "return all records after and including the cursor" in {
    // This test checks that the result is implicitly ordered by ID ASC to guarantee a stable ordering of results, because a non-unique field
    // can't guarantee a stable ordering in itself.
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, orderBy: { field: DESC }) {
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

  "A cursor (on ID) query with an ascending order on a non-unique field" should "return all records after and including the cursor" in {
    // This test checks that the result is implicitly ordered by ID ASC to guarantee a stable ordering of results, because a non-unique field
    // can't guarantee a stable ordering in itself.
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, orderBy: { field: ASC }) {
          |    id
          |    field
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be(
      """{"data":{"findManyTestModel":[{"id":5,"field":"Field5"},{"id":6,"field":"Field5"},{"id":7,"field":"Field7"},{"id":8,"field":"Field7"},{"id":9,"field":"Field9"},{"id":10,"field":"Field9"}]}}""")
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
          |  }, orderBy: { id: DESC }) {
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

  /*********************
    * Take only tests. *
    ********************/
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
          |  findManyTestModel(take: 1, orderBy: { id: DESC }) {
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
          |  findManyTestModel(take: -1, orderBy: { id: ASC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":10}]}}""")
  }

  /*********************
    * Skip only tests. *
    ********************/
  "A skip" should "return all records after the offset specified" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(skip: 5, orderBy: { id: ASC }) {
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
          |  findManyTestModel(skip: 5, orderBy: { id: DESC }) {
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

  "Skipping 0 records" should "return all records beginning from the first" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(skip: 0, orderBy: { id: ASC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5},{"id":6},{"id":7},{"id":8},{"id":9},{"id":10}]}}""")
  }

  /*************************
    * Cursor + Take tests. *
    ************************/
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
          |  }, take: -2, orderBy: { id: ASC }) {
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
          |  }, take: 2, orderBy: { id: DESC }) {
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
          |  }, take: -2, orderBy: { id: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}""")
  }

  /********************************
    * Cursor + Take + Skip tests. *
    *******************************/
  "A cursor with take 2 and skip 2" should "return 2 records after the next record after the cursor" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: 2, skip: 2) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":7},{"id":8}]}}""")
  }

  "A cursor with take -2 and skip 2" should "return 2 records before the previous record of the cursor" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: -2, skip: 2, orderBy: { id: ASC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}""")
  }

  "Skipping to the end with take" should "return no records" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 9
          |  }, take: 2, skip: 2) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  "A cursor with take 0 and skip" should "return no records" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 1
          |  }, skip: 1, take: 0) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[]}}""")
  }

  "A cursor with take 2, skip 2 and reversed ordering" should "return 2 records before the record before the cursor (in the original ordering)" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: 2, skip: 2, orderBy: { id: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":3},{"id":2}]}}""")
  }

  "A cursor with take -2, skip 2 and reversed ordering" should "return 2 records after the record before the cursor (in the original ordering)" in {
    val data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: {
          |    id: 5
          |  }, take: -2, skip: 2, orderBy: { id: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    data.toString() should be("""{"data":{"findManyTestModel":[{"id":8},{"id":7}]}}""")
  }

  /*************************************************
    * Cursor + Take + Skip + Multiple OrderBy tests. *
    * ************************************************/
  "A cursor with take, skip and multiple order-bys with the orderBy combination stable" should "return the expected results generalized over more than 2 orderBys" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model TestModel {
        |  id     Int    @id
        |  fieldA String
        |  fieldB String
        |  fieldC String
        |  fieldD String
        |
        |  @@unique([fieldA, fieldB, fieldC, fieldD])
        |}
      """.stripMargin
    }
    database.setup(project)

    // Test data:
    // All fields combined are a unique combination (guarantee stable ordering).
    //
    // ID   fieldA fieldB fieldC fieldD
    // 1 =>    A      B      C      D
    // 2 =>    A      A      A      B
    // 3 =>    B      B      B      B
    // 4 =>    B      B      B      C
    // 5 =>    C      C      B      A
    // 6 =>    C      C      D      C
    server.query("""mutation {createOneTestModel(data: { id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 2, fieldA: "A", fieldB: "A", fieldC: "A", fieldD: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 3, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 4, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "C"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 5, fieldA: "C", fieldB: "C", fieldC: "B", fieldD: "A"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 6, fieldA: "C", fieldB: "C", fieldC: "D", fieldD: "C"}){ id }}""", project, legacy = false)

    // >>> TEST #1
    var data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: { id: 4 }, take: 2, skip: 1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 5 => C C B A
    // 6 => C C D C
    // 4 => B B B C <- cursor, skipped
    // 3 => B B B B <- take
    // 2 => A A A B <- take
    // 1 => A B C D
    data.toString() should be("""{"data":{"findManyTestModel":[{"id":3},{"id":2}]}}""")

    // >>> TEST #2
    data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: { id: 4 }, take: 2, skip: 1, orderBy: { fieldA: ASC, fieldB: DESC, fieldC: DESC, fieldD: ASC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered (reverse from test #1): ASC, DESC, DESC, ASC
    // 1 => A B C D
    // 2 => A A A B
    // 3 => B B B B
    // 4 => B B B C <- cursor, skipped
    // 6 => C C D C <- take
    // 5 => C C B A <- take
    data.toString() should be("""{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}""")

    // Note: Negative takes reverse the order, the following tests check that.

    // >>> TEST #3, same order as 1, but gets reversed to test 2
    data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: { id: 4 }, take: -2, skip: 1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Originally the query orders: DESC, ASC, ASC, DESC. With -2 instead of 2, it wants to take:
    // 5 => C C B A <- take
    // 6 => C C D C <- take
    // 4 => B B B C <- cursor, skipped
    // 3 => B B B B
    // 2 => A A A B
    // 1 => A B C D
    //
    // The connectors reverse this to (equivalent to test #2): ASC, DESC, DESC, ASC
    // 1 => A B C D
    // 2 => A A A B
    // 3 => B B B B
    // 4 => B B B C <- cursor, skipped
    // 6 => C C D C <- take
    // 5 => C C B A <- take
    //
    // Because the final result (6, 5) gets reversed again to restore original order, the result is:
    data.toString() should be("""{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}""")
  }

  "A cursor with take, skip and multiple order-bys with the orderBy combination not stable" should "return the expected results" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model TestModel {
        |  id     Int    @id
        |  fieldA String
        |  fieldB String
        |  fieldC String
        |  fieldD String
        |}
      """.stripMargin
    }
    database.setup(project)

    // Test data:
    // No stable ordering guaranteed.
    //
    // ID   fieldA fieldB fieldC fieldD
    // 1 =>    A      B      C      D
    // 2 =>    A      A      A      B
    // 3 =>    B      B      B      B
    // 4 =>    B      B      B      B
    // 5 =>    B      B      B      B
    // 6 =>    C      C      D      C
    server.query("""mutation {createOneTestModel(data: { id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 2, fieldA: "A", fieldB: "A", fieldC: "A", fieldD: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 3, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 4, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 5, fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}){ id }}""", project, legacy = false)
    server.query("""mutation {createOneTestModel(data: { id: 6, fieldA: "C", fieldB: "C", fieldC: "D", fieldD: "C"}){ id }}""", project, legacy = false)

    // >>> TEST #1
    var data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: { id: 4 }, take: 3, skip: 1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // The order is at the discretion of the db, possible result options:
    // - 3 and 5 are included in the result: (3, 5, 2) | (5, 3, 2)
    // - Only 3 or only 5 are included in the result: (3, 2, 1) | (5, 2, 1)
    // - None of the duplicates is included: (2, 1)
    //
    // One possible query constellation:
    // 6 => C C D C
    // 5 => B B B B
    // 4 => B B B B <- cursor, skipped
    // 3 => B B B B <- take
    // 2 => A A A B <- take
    // 1 => A B C D <- take
    var possible_results = Seq(
      """{"data":{"findManyTestModel":[{"id":3},{"id":5},{"id":2}]}}""",
      """{"data":{"findManyTestModel":[{"id":5},{"id":3},{"id":2}]}}""",
      """{"data":{"findManyTestModel":[{"id":3},{"id":2},{"id":1}]}}""",
      """{"data":{"findManyTestModel":[{"id":5},{"id":2},{"id":1}]}}""",
      """{"data":{"findManyTestModel":[{"id":2},{"id":1}]}}"""
    )

    possible_results should contain(data.toString())

    // >>> TEST #2
    data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: { id: 4 }, take: 3, skip: 1, orderBy: { fieldA: ASC, fieldB: DESC, fieldC: DESC, fieldD: ASC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered (reverse from test #1): ASC, DESC, DESC, ASC
    // The order is at the discretion of the db, possible result options (cursor on 4):
    // - 3 and 5 are included in the result: (3, 5, 6) | (5, 3, 6)
    // - Only 3 or only 5 are included in the result: (3, 6) | (5, 6)
    // - None of the duplicates is included: (6)
    //
    // One possible query constellation:
    // 1 => A B C D
    // 2 => A A A B
    // 4 => B B B B <- cursor, skipped
    // 3 => B B B B <- take
    // 5 => B B B B <- take
    // 6 => C C D C <- take
    possible_results = Seq(
      """{"data":{"findManyTestModel":[{"id":3},{"id":5},{"id":6}]}}""",
      """{"data":{"findManyTestModel":[{"id":5},{"id":3},{"id":6}]}}""",
      """{"data":{"findManyTestModel":[{"id":3},{"id":6}]}}""",
      """{"data":{"findManyTestModel":[{"id":5},{"id":6}]}}""",
      """{"data":{"findManyTestModel":[{"id":6}]}}"""
    )

    possible_results should contain(data.toString())

    // Note: Negative takes reverse the order, the following tests check that.

    // >>> TEST #3, same order as 1, but gets reversed to test 2
    data = server
      .query(
        """
          |query {
          |  findManyTestModel(cursor: { id: 4 }, take: -3, skip: 1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |    id
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Originally the query orders: DESC, ASC, ASC, DESC (equivalent to test #1).
    // With -3 instead of 3, it wants to take (possibility):
    // 6 => C C D C <- take
    // 5 => B B B B <- take
    // 3 => B B B B <- take
    // 4 => B B B B <- cursor, skipped
    // 2 => A A A B
    // 1 => A B C D
    //
    // The connectors reverse this to (equivalent to test #2): ASC, DESC, DESC, ASC
    // 1 => A B C D
    // 2 => A A A B
    // 4 => B B B B <- cursor, skipped
    // 3 => B B B B <- take
    // 5 => B B B B <- take
    // 6 => C C D C <- take
    //
    // Because the final result gets reversed again to restore original order, the result possibilities are the same as #2, just reversed.
    possible_results = Seq(
      """{"data":{"findManyTestModel":[{"id":6},{"id":5},{"id":3}]}}""",
      """{"data":{"findManyTestModel":[{"id":6},{"id":3},{"id":5}]}}""",
      """{"data":{"findManyTestModel":[{"id":6},{"id":3}]}}""",
      """{"data":{"findManyTestModel":[{"id":6},{"id":5}]}}""",
      """{"data":{"findManyTestModel":[{"id":6}]}}"""
    )

    possible_results should contain(data.toString())
  }
}
