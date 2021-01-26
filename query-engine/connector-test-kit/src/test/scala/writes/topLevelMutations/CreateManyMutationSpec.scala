package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsValue
import util.ConnectorCapability.EnumCapability
import util._

class CreateManyMutationSpec extends FlatSpec with Matchers with ApiSpecBase {
  // Covers: Autoincrement working, basic functionality.
  "A basic createMany" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id   Int @id @default(autoincrement())
        |  str1 String
        |  str2 String?
        |  str3 String? @default("SOME_DEFAULT")
        |}
        |""".stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createManyTest(data: [
        |    { id: 123, str1: "1", str2: "1", str3: "1"},
        |    { id: 321, str1: "2",            str3: null},
        |    {          str1: "1"},
        |  ]) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":3}}}""")
  }

  "createMany" should "correctly use defaults and nulls" in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id  Int @id
        |  str String? @default("SOME_DEFAULT")
        |}
        |""".stripMargin
    }
    database.setup(project)

    // Not providing a value must provide the default, providing null must result in null.
    val result = server.query(
      """
        |mutation {
        |  createManyTest(data: [
        |    { id: 1 },
        |    { id: 2, str: null }
        |  ]) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":2}}}""")

    val check = server.query(
      """
        |{
        |  findManyTest {
        |    id
        |    str
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    check.toString() should be("""{"data":{"findManyTest":[{"id":1,"str":"SOME_DEFAULT"},{"id":2,"str":null}]}}""")
  }

  "createMany" should "error on duplicates by default" in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id  Int @id
        |}
        |""".stripMargin
    }
    database.setup(project)

    server.queryThatMustFail(
      """
        |mutation {
        |  createManyTest(data: [
        |    { id: 1 },
        |    { id: 1 }
        |  ]) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      errorCode = 2002,
      errorContains = "UniqueConstraintViolation",
      legacy = false
    )
  }

  "createMany" should "not error on duplicates with skipDuplicates true" in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id  Int @id
        |}
        |""".stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createManyTest(skipDuplicates: true, data: [
        |    { id: 1 },
        |    { id: 1 }
        |  ]) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":1}}}""")
  }

  // Covers: Batching by row number.
  // 5000 params are allowed per single query, but only 2500 rows at once
  // Each created row has 1 param and we create 10000 records, so 4 queries will be fired.
  // Note: Validation of the correct batching behavior is done manually.
  "createMany" should "allow creating a large number of records (horizontal partitioning check)" in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id  Int @id
        |}
        |""".stripMargin
    }
    database.setup(project)

    val records: Seq[String] = for (i <- 1 to 10000) yield { s"{ id: $i }" }
    val result = server.query(
      s"""
        |mutation {
        |  createManyTest(skipDuplicates: true, data: [${records.mkString(",")}]) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":10000}}}""")
  }

  // Covers: Batching by row number.
  // 5000 params are allowed per single query, but only 2500 rows at once
  // Each created row has 5 params and we create 2000 rows, so 2 queries will be fired.
  // Note: Validation of the correct batching behavior is done manually.
  "createMany" should "allow creating a large number of records (vertical partitioning check)" in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id Int @id
        |  a  Int
        |  b  Int
        |  c  Int
        |}
        |""".stripMargin
    }
    database.setup(project)

    val records: Seq[String] = for (i <- 1 to 2000) yield { s"{ id: $i, a: $i, b: $i, c: $i }" }
    val result = server.query(
      s"""
         |mutation {
         |  createManyTest(skipDuplicates: true, data: [${records.mkString(",")}]) {
         |    count
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":2000}}}""")
  }
}
