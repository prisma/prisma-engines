package writes.nestedMutations.notUsingSchemaBase

import org.scalatest.{FlatSpec, Matchers}
import util._

class NestedCreateManyMutationSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  val project = ProjectDsl.fromString {
    """
      |model ModelA {
      |  id Int      @id
      |  bs ModelB[]
      |}
      |
      |model ModelB {
      |  id   Int     @id
      |  str1 String
      |  str2 String?
      |  str3 String? @default("SOME_DEFAULT")
      |  a_id Int?
      |  a    ModelA? @relation(fields: [a_id], references: [id])
      |}
      |""".stripMargin
  }

  override def beforeEach: Unit = {
    database.setup(project)
    database.truncateProjectTables(project)
    super.beforeEach()
  }

  "A basic createMany on a create top level" should "work" taggedAs IgnoreSQLite in {
    val result = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1,
        |    bs: {
        |      createMany: {
        |        skipDuplicates: false,
        |        data: [
        |          { id: 1, str1: "1", str2: "1", str3: "1"},
        |          { id: 2, str1: "2",            str3: null},
        |          { id: 3, str1: "1"},
        |        ]
        |      }
        |    }
        |  }) {
        |    bs {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createOneModelA":{"bs":[{"id":1},{"id":2},{"id":3}]}}}""")
  }

  "Nested createMany" should "error on duplicates by default" taggedAs IgnoreSQLite in {
    server.queryThatMustFail(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1,
        |    bs: {
        |      createMany: {
        |        data: [
        |          { id: 1, str1: "1", str2: "1", str3: "1"},
        |          { id: 1, str1: "2",            str3: null},
        |        ]
        |      }
        |    }
        |  }) {
        |    bs {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2002,
      errorContains = "UniqueConstraintViolation",
      legacy = false
    )
  }

  "Nested createMany" should "not error on duplicates with skipDuplicates true" taggedAs (IgnoreSQLite, IgnoreMsSql) in {
    val result = server.query(
      """
        |mutation {
        |  createOneModelA(data: {
        |    id: 1,
        |    bs: {
        |      createMany: {
        |        skipDuplicates: true,
        |        data: [
        |          { id: 1, str1: "1", str2: "1", str3: "1"},
        |          { id: 1, str1: "2",            str3: null},
        |        ]
        |      }
        |    }
        |  }) {
        |    bs {
        |      id
        |    }
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createOneModelA":{"bs":[{"id":1}]}}}""")
  }

  // Note: Checks were originally higher, but test method (command line args) blows up...
  // Covers: Batching by row number.
  // Each DB allows a certain amount of params per single query, and a certain number of rows.
  // We create 1000 nested records.
  "Nested createMany" should "allow creating a large number of records (horizontal partitioning check)" taggedAs IgnoreSQLite in {
    val records: Seq[String] = for (i <- 1 to 1000) yield { s"""{ id: $i, str1: "$i" }""" }
    server.query(
      s"""
         |mutation {
         |  createOneModelA(data: {
         |    id: 1
         |    bs: {
         |      createMany: {
         |        data: [${records.mkString(",")}]
         |      }
         |    }
         |  }) {
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false
    )

    val result = server.query(
      s"""
         |{
         |  aggregateModelB {
         |    count {
         |      _all
         |    }
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"aggregateModelB":{"count":{"_all":1000}}}}""")
  }
}
