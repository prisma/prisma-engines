package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsValue
import util.ConnectorCapability.EnumCapability
import util._

// RS: Ported
class CreateManyMutationSpec extends FlatSpec with Matchers with ApiSpecBase {
  "A basic createMany" should "work" taggedAs IgnoreSQLite in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id   Int @id
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
        |    { id: 1, str1: "1", str2: "1", str3: "1"},
        |    { id: 2, str1: "2",            str3: null},
        |    { id: 3, str1: "1"},
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

  // Covers: Autoincrement ID working with basic functionality.
  "A basic createMany with autoincrementid " should "work" taggedAs (IgnoreSQLite, IgnoreMsSql) in {
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

  "createMany" should "correctly use defaults and nulls" taggedAs IgnoreSQLite in {
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

  "createMany" should "error on duplicates by default" taggedAs IgnoreSQLite in {
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

  "createMany" should "not error on duplicates with skipDuplicates true" taggedAs (IgnoreSQLite, IgnoreMsSql) in {
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

  // Note: Checks were originally higher, but test method (command line args) blows up...
  // Covers: Batching by row number.
  // Each DB allows a certain amount of params per single query, and a certain number of rows.
  // Each created row has 1 param and we create 1000 records.
  "createMany" should "allow creating a large number of records (horizontal partitioning check)" taggedAs IgnoreSQLite in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id  Int @id
        |}
        |""".stripMargin
    }
    database.setup(project)

    val records: Seq[String] = for (i <- 1 to 1000) yield { s"{ id: $i }" }
    val result = server.query(
      s"""
        |mutation {
        |  createManyTest(data: [${records.mkString(",")}]) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":1000}}}""")
  }

  // Note: Checks were originally higher, but test method (command line args) blows up...
  // Covers: Batching by row number.
  // Each DB allows a certain amount of params per single query, and a certain number of rows.
  // Each created row has 4 params and we create 1000 rows.
  "createMany" should "allow creating a large number of records (vertical partitioning check)" taggedAs IgnoreSQLite in {
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
         |  createManyTest(data: [${records.mkString(",")}]) {
         |    count
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createManyTest":{"count":2000}}}""")
  }

  "createMany" should "not be available on SQLite" taggedAs (IgnoreMsSql, IgnoreMySql, IgnorePostgres) in {
    val project = ProjectDsl.fromString {
      """
        |model Test {
        |  id Int @id
        |}
        |""".stripMargin
    }
    database.setup(project)

    server.queryThatMustFail(
      s"""
         |mutation {
         |  createManyTest(data: []) {
         |    count
         |  }
         |}
      """.stripMargin,
      project,
      errorContains = "`Field does not exist on enclosing type.` at `Mutation.createManyTest`",
      errorCode = 2009,
    )
  }
}
