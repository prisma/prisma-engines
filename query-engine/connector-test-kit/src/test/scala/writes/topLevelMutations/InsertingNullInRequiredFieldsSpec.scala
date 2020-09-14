package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util._

class InsertingNullInRequiredFieldsSpec extends FlatSpec with Matchers with ApiSpecBase {

  "Updating a required value to null" should "throw a proper error" in {
    val project = ProjectDsl.fromString {
      """model A {
        |  id  String @id @default(cuid())
        |  b   String @unique
        |  key String
        |}
      """
    }
    database.setup(project)

    server.query(
      """mutation a {
        |  createA(data: {
        |    b: "abc"
        |    key: "abc"
        |  }) {
        |    id
        |  }
        |}""",
      project
    )

    if (connectorConfig.name != "mysql56") {
      server.queryThatMustFail(
        """mutation b {
            |  updateA(
            |    where: { b: "abc" }
            |    data: {
            |      key: { set: null }
            |    }) {
            |    id
            |  }
            |}""",
        project,
        errorCode = 2009,
        errorContains = "`Mutation.updateA.data.AUpdateInput.key.StringFieldUpdateOperationsInput.set`: A value is required but not set."
      )
    }
  }

  "Creating a required value as null" should "throw a proper error" in {
    val project = ProjectDsl.fromString {
      """model A {
        |  id  String  @id @default(cuid())
        |  b   String  @unique
        |  key String
        |}
      """
    }
    database.setup(project)

    server.queryThatMustFail(
      """mutation a {
        |  createA(data: {
        |    b: "abc"
        |    key: null
        |  }) {
        |    id
        |  }
        |}""",
      project,
      errorCode = 2012,
      errorContains = """Missing a required value at `Mutation.createA.data.ACreateInput.key"""
    )
  }

  "Updating an optional value to null" should "work" in {
    val project = ProjectDsl.fromString {
      """model A {
        |  id  String  @id @default(cuid())
        |  b   String  @unique
        |  key String? @unique
        |}
      """
    }
    database.setup(project)

    server.query(
      """mutation a {
        |  createA(data: {
        |    b: "abc"
        |    key: "abc"
        |  }) {
        |    id
        |  }
        |}""",
      project
    )

    server.query(
      """mutation b {
        |  updateA(
        |    where: { b: "abc" }
        |    data: {
        |      key: { set: null }
        |    }) {
        |    id
        |  }
        |}""",
      project
    )

    server.query("""query{as{b,key}}""", project).toString should be("""{"data":{"as":[{"b":"abc","key":null}]}}""")
  }

  "Creating an optional value as null" should "work" in {
    val project = ProjectDsl.fromString {
      """model A {
        |  id   String @id @default(cuid())
        |  b    String @unique
        |  key  String?
        |}
      """
    }
    database.setup(project)

    server.query(
      """mutation a {
        |  createA(data: {
        |    b: "abc"
        |    key: null
        |  }) {
        |    b,
        |    key
        |  }
        |}""",
      project,
      dataContains = """{"createA":{"b":"abc","key":null}}"""
    )
  }

}
