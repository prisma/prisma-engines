package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsValue
import util.ConnectorCapability.EnumCapability
import util._

class CreateMutationSpec extends FlatSpec with Matchers with ApiSpecBase {

  override def runOnlyForCapabilities = Set(EnumCapability)

  val schema =
    """
    |model ScalarModel {
    |    id          String @id @default(cuid())
    |    optString   String?
    |    optInt      Int?
    |    optFloat    Float?
    |    optBoolean  Boolean?
    |    optEnum     MyEnum?
    |    optDateTime DateTime?
    |    optUnique   String? @unique
    |    createdAt   DateTime @default(now())
    |    relId       String?
    |    optRel      RelatedModel? @relation(fields: [relId], references: [id])
    |}
    |
    |model RelatedModel {
    |    id String @id @default(cuid())
    |}
    |
    |enum MyEnum {
    |   A
    |   B
    |}""".stripMargin

  val project = ProjectDsl.fromString { schema }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "A Create Mutation" should "create and return item" in {
    val res = server.query(
      s"""mutation {
         |  createScalarModel(data: {
         |    optString: "lala${TroubleCharacters.value}", optInt: 1337, optFloat: 1.234, optBoolean: true, optEnum: A, optDateTime: "2016-07-31T23:59:01.000Z"
         |  }){id, optString, optInt, optFloat, optBoolean, optEnum, optDateTime }
         |}""".stripMargin,
      project = project
    )
    val id = res.pathAsString("data.createScalarModel.id")

    res should be(
      s"""{"data":{"createScalarModel":{"id":"$id","optInt":1337,"optBoolean":true,"optDateTime":"2016-07-31T23:59:01+00:00","optString":"lala${TroubleCharacters.value}","optEnum":"A","optFloat":1.234}}}""".parseJson)

    val queryRes = server.query("""{ scalarModels{optString, optInt, optFloat, optBoolean, optEnum, optDateTime }}""", project = project)

    queryRes should be(
      s"""{"data":{"scalarModels":[{"optInt":1337,"optBoolean":true,"optDateTime":"2016-07-31T23:59:01+00:00","optString":"lala${TroubleCharacters.value}","optEnum":"A","optFloat":1.234}]}}""".parseJson)
  }

  "A Create Mutation" should "create and return item with empty string" in {
    val res = server.query(
      """mutation {
        |  createScalarModel(data: {
        |    optString: ""
        |  }){optString, optInt, optFloat, optBoolean, optEnum }}""".stripMargin,
      project = project
    )

    res should be("""{"data":{"createScalarModel":{"optInt":null,"optBoolean":null,"optString":"","optEnum":null,"optFloat":null}}}""".parseJson)
  }

  "A Create Mutation" should "create and return item with explicit null attributes" in {
    val res = server.query(
      """mutation {
        |  createScalarModel(data: {
        |    optString: null, optInt: null, optBoolean: null, optEnum: null, optFloat: null
        |  }){optString, optInt, optFloat, optBoolean, optEnum}}""".stripMargin,
      project
    )

    res should be("""{"data":{"createScalarModel":{"optInt":null,"optBoolean":null,"optString":null,"optEnum":null,"optFloat":null}}}""".parseJson)
  }

  "A Create Mutation" should "create and return item with explicit null attributes when other mutation has explicit non-null values" in {
    {
      val res = server.query(
        """mutation {
          | createScalarModel(data: {optString: "lala", optInt: 123, optBoolean: true, optEnum: A, optFloat: 1.23}){optString, optInt, optFloat, optBoolean, optEnum }
          |}""".stripMargin,
        project = project
      )

      res.pathAs[JsValue]("data.createScalarModel") should be("""{"optInt":123,"optBoolean":true,"optString":"lala","optEnum":"A","optFloat":1.23}""".parseJson)
    }

    {
      val res = server.query(
        """mutation {
          | createScalarModel(data: {optString: null, optInt: null, optBoolean: null, optEnum: null, optFloat: null}){optString, optInt, optFloat, optBoolean, optEnum }
          |}""".stripMargin,
        project = project
      )

      res.pathAs[JsValue]("data.createScalarModel") should be("""{"optInt":null,"optBoolean":null,"optString":null,"optEnum":null,"optFloat":null}""".parseJson)
    }
  }

  "A Create Mutation" should "create and return item with implicit null attributes and createdAt should be set" in {
    val res = server.query("""mutation {createScalarModel(data:{}){ optString, optInt, optFloat, optBoolean, optEnum }}""", project)

    // if the query succeeds createdAt did work. If would not have been set we would get a NullConstraintViolation.
    res should be("""{"data":{"createScalarModel":{"optInt":null,"optBoolean":null,"optString":null,"optEnum":null,"optFloat":null}}}""".parseJson)
  }

  "A Create Mutation" should "fail when a DateTime is invalid" in {
    server.queryThatMustFail(
      s"""mutation { createScalarModel(data:
         |  { optString: "test", optInt: 1337, optFloat: 1.234, optBoolean: true, optEnum: A, optDateTime: "2016-0B-31T23:59:01.000Z" }
         |  ){optString, optInt, optFloat, optBoolean, optEnum, optDateTime}}""".stripMargin,
      project = project,
      0,
      errorContains =
        "`Mutation.createScalarModel.data.ScalarModelCreateInput.optDateTime`: Error parsing value: Invalid DateTime: '2016-0B-31T23:59:01.000Z' (must be ISO 8601 compatible). Underlying error: input contains invalid characters."
    )
  }

  "A Create Mutation" should "fail when an Int is invalid" in {
    server.queryThatMustFail(
      s"""mutation {createScalarModel(data: {optString: "test", optInt: B, optFloat: 1.234, optBoolean: true, optEnum: A, optDateTime: "2016-07-31T23:59:01.000Z" }){optString, optInt, optFloat, optBoolean, optEnum, optDateTime }}""",
      project = project,
      errorCode = 2009,
      errorContains =
        """Query parsing/validation error at `Mutation.createScalarModel.data.ScalarModelCreateInput.optInt`: Value types mismatch. Have: Enum(\"B\"), want: Int""",
    )
  }

  "A Create Mutation" should "gracefully fail when a unique violation occurs" in {
    val mutation = s"""mutation {createScalarModel(data: {optUnique: "test"}){optUnique}}"""
    server.query(mutation, project)
    server.queryThatMustFail(mutation, project, errorCode = 2002) // 3010)
  }

  "A Create Mutation" should "create and return an item with enums passed as strings" in {
    val res = server.query(s"""mutation {createScalarModel(data: {optEnum: "A"}){ optEnum }}""", project)
    res should be("""{"data":{"createScalarModel":{"optEnum":"A"}}}""".parseJson)
  }

  "A Create Mutation" should "fail if an item with enums passed as strings doesn't match and enum value" in {
    // previous errorCode: 3010
    server.queryThatMustFail(
      s"""mutation {createScalarModel(data: {optEnum: "NOPE"}){ optEnum }}""",
      project,
      errorCode = 2009,
      errorContains =
        """Query parsing/validation error at `Mutation.createScalarModel.data.ScalarModelCreateInput.optEnum`: Error parsing value: Enum value 'NOPE' is invalid for enum type MyEnum."""
    )
  }

  "A Create Mutation" should "reject an optional relation set to null." in {
    server.queryThatMustFail(
      """mutation {
        |  createScalarModel(data: {
        |    optRel: null
        |  }){ relId }}""".stripMargin,
      project = project,
      errorCode = 2012,
      errorContains = "Missing a required value at `Mutation.createScalarModel.data.ScalarModelCreateInput.optRel`"
    )
  }

  "A Create Mutation" should "create with an optional relation omitted." in {
    val res = server.query(
      """mutation {
        |  createScalarModel(data: {}) {
        |    relId
        |  }}""".stripMargin,
      project = project
    )

    res should be("""{"data":{"createScalarModel":{"relId":null}}}""".parseJson)
  }
}
