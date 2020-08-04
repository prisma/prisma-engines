package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.Json
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedMultiOrderPaginationSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Paging on an 1:m relation with a multi-field orderBy with stable ordering" should "work as expected" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model TestModel {
        |  id      Int @id
        |  related RelatedTestModel[]
        |}
        |
        |model RelatedTestModel {
        |  id Int @id
        |  fieldA String
        |  fieldB String
        |  fieldC String
        |  fieldD String
        |
        |  parent_id Int
        |  parent TestModel @relation(fields: [parent_id], references: [id])
        |
        |  @@unique([fieldA, fieldB, fieldC, fieldD])
        |}
      """.stripMargin
    }
    database.setup(project)

    // Test data:
    // All fields combined are a unique combination (guarantees stable ordering).
    //
    // Parent Child fieldA fieldB fieldC fieldD
    //    1  =>  1      A      B      C      D
    //    1  =>  2      B      B      B      B
    //    2  =>  3      B      A      B      B
    //    2  =>  4      B      B      B      C
    //    3  =>  5      C      C      B      A
    //    3  =>  6      A      C      D      C
    server.query(
      """mutation { createOneTestModel(data: { id: 1, related: { create: [{ id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}, { id: 2,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}]}}){ id } }""",
      project,
      legacy = false
    )
    server.query(
      """mutation { createOneTestModel(data: { id: 2, related: { create: [{ id: 3,  fieldA: "B", fieldB: "A", fieldC: "B", fieldD: "B"},{ id: 4,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "C"}]}}){ id } }""",
      project,
      legacy = false
    )
    server.query(
      """mutation { createOneTestModel(data: { id: 3, related: { create: [{ id: 5, fieldA: "C", fieldB: "C", fieldC: "B", fieldD: "A"},{ id: 6,  fieldA: "A", fieldB: "C", fieldC: "D", fieldD: "C"}]}}){ id } }""",
      project,
      legacy = false
    )

    // >>> TEST #1, take only the first child of each parent
    var data = server
      .query(
        """
          |query {
          |  findManyTestModel {
          |    id
          |    related(take: 1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |      id
          |    }
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 1 => 2 B B B B <- take
    // 1 => 1 A B C D
    // 2 => 3 B A B B <- take
    // 2 => 4 B B B C
    // 3 => 5 C C B A <- take
    // 3 => 6 A C D C
    // Makes: [1 => 2, 2 => 3, 3 => 5]
    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1,"related":[{"id":2}]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[{"id":5}]}]}}""")

    // >>> TEST #2, take last child of each parent
    data = server
      .query(
        """
          |query {
          |  findManyTestModel {
          |    id
          |    related(take: -1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |      id
          |    }
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 1 => 2 B B B B
    // 1 => 1 A B C D <- take
    // 2 => 3 B A B B
    // 2 => 4 B B B C <- take
    // 3 => 5 C C B A
    // 3 => 6 A C D C <- take
    // Makes: [1 => 1, 2 => 4, 3 => 6]
    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1,"related":[{"id":1}]},{"id":2,"related":[{"id":4}]},{"id":3,"related":[{"id":6}]}]}}""")

    // >>> TEST #3, cursor on child 3
    data = server
      .query(
        """
          |query {
          |  findManyTestModel {
          |    id
          |    related(cursor: { id: 3 }, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |      id
          |    }
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 1 => 2 B B B B
    // 1 => 1 A B C D
    // 2 => 3 B A B B <- take
    // 2 => 4 B B B C <- take
    // 3 => 5 C C B A
    // 3 => 6 A C D C
    // Makes: [1 => [], 2 => [3, 4], 3 => []]
    data.toString() should be("""{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":3},{"id":4}]},{"id":3,"related":[]}]}}""")
  }

  "Paging on an 1:m relation with a multi-field orderBy without stable ordering" should "work as expected" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model TestModel {
        |  id      Int @id
        |  related RelatedTestModel[]
        |}
        |
        |model RelatedTestModel {
        |  id Int @id
        |  fieldA String
        |  fieldB String
        |  fieldC String
        |  fieldD String
        |
        |  parent_id Int
        |  parent TestModel @relation(fields: [parent_id], references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    // Test data:
    // No stable ordering guaranteed.
    //
    // Parent Child fieldA fieldB fieldC fieldD
    //    1  =>  1      A      B      C      D
    //    1  =>  2      B      B      B      B
    //    2  =>  3      B      B      B      B
    //    2  =>  4      B      B      B      B
    //    3  =>  5      C      C      B      A
    //    3  =>  6      A      C      D      C
    server.query(
      """mutation { createOneTestModel(data: { id: 1, related: { create: [{ id: 1, fieldA: "A", fieldB: "B", fieldC: "C", fieldD: "D"}, { id: 2,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}]}}){ id } }""",
      project,
      legacy = false
    )
    server.query(
      """mutation { createOneTestModel(data: { id: 2, related: { create: [{ id: 3,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"},{ id: 4,  fieldA: "B", fieldB: "B", fieldC: "B", fieldD: "B"}]}}){ id } }""",
      project,
      legacy = false
    )
    server.query(
      """mutation { createOneTestModel(data: { id: 3, related: { create: [{ id: 5, fieldA: "C", fieldB: "C", fieldC: "B", fieldD: "A"},{ id: 6,  fieldA: "A", fieldB: "C", fieldC: "D", fieldD: "C"}]}}){ id } }""",
      project,
      legacy = false
    )

    // >>> TEST #1, take only the first child of each parent
    var data = server
      .query(
        """
          |query {
          |  findManyTestModel {
          |    id
          |    related(take: 1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |      id
          |    }
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 1 => 2 B B B B <- take
    // 1 => 1 A B C D
    // 2 => 3 B B B B <- take
    // 2 => 4 B B B B <- xor take
    // 3 => 5 C C B A <- take
    // 3 => 6 A C D C
    // Makes: [1 => 2, 2 => 3 | 4, 3 => 5]
    var possible_results = Seq(
      """{"data":{"findManyTestModel":[{"id":1,"related":[{"id":2}]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[{"id":5}]}]}}""",
      """{"data":{"findManyTestModel":[{"id":1,"related":[{"id":2}]},{"id":2,"related":[{"id":4}]},{"id":3,"related":[{"id":5}]}]}}""",
    )

    possible_results should contain(data.toString())

    // >>> TEST #2, take last child of each parent
    data = server
      .query(
        """
          |query {
          |  findManyTestModel {
          |    id
          |    related(take: -1, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |      id
          |    }
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 1 => 2 B B B B
    // 1 => 1 A B C D <- take
    // 2 => 3 B B B B <- take
    // 2 => 4 B B B B <- xor take
    // 3 => 5 C C B A
    // 3 => 6 A C D C <- take
    // Makes: [1 => 1, 2 => 4, 3 => 6]
    possible_results = Seq(
      """{"data":{"findManyTestModel":[{"id":1,"related":[{"id":1}]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[{"id":6}]}]}}""",
      """{"data":{"findManyTestModel":[{"id":1,"related":[{"id":1}]},{"id":2,"related":[{"id":4}]},{"id":3,"related":[{"id":6}]}]}}""",
    )

    possible_results should contain(data.toString())

    // >>> TEST #3, cursor on child 3
    data = server
      .query(
        """
          |query {
          |  findManyTestModel {
          |    id
          |    related(cursor: { id: 3 }, orderBy: { fieldA: DESC, fieldB: ASC, fieldC: ASC, fieldD: DESC }) {
          |      id
          |    }
          |  }
          |}
        """,
        project,
        legacy = false
      )

    // Ordered: DESC, ASC, ASC, DESC
    // 1 => 2 B B B B
    // 1 => 1 A B C D
    // 2 => 3 B B B B <- take
    // 2 => 4 B B B B <- take
    // 3 => 5 C C B A
    // 3 => 6 A C D C
    // Makes: [1 => [], 2 => [3, 4] | [4, 3] | [3] | [4], 3 => []]
    possible_results = Seq(
      """{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":3},{"id":4}]},{"id":3,"related":[]}]}}""",
      """{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":4},{"id":3}]},{"id":3,"related":[]}]}}""",
      """{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":3}]},{"id":3,"related":[]}]}}""",
      """{"data":{"findManyTestModel":[{"id":1,"related":[]},{"id":2,"related":[{"id":4}]},{"id":3,"related":[]}]}}""",
    )

    possible_results should contain(data.toString())
  }
}
