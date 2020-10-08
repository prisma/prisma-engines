package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NonUniqueIndexUsageSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  // Validates fix for
  // https://github.com/prisma/prisma/issues/3869
  // https://github.com/prisma/prisma-client-js/issues/71

  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "Non-unique indices" should "not cause unique filters for that field to show up" in {
    val project = ProjectDsl.fromString {
      s"""
           |model TestModel {
           |  id    Int     @id
           |  field String?
           |
           |  @@index([field], name: "test_index")
           |}
       """
    }
    database.setup(project)

    server.query(
      s"""mutation {
         |createOneTestModel(data: { id: 1, field: "Test" }) {
         |    id
         |  }
         |}
      """,
      project,
      legacy = false
    )

    // Field must not show up in the *WhereUniqueInput.
    server.queryThatMustFail(
      s"""query {
         |findOneTestModel(where: { field: "nope" }) {
         |    id
         |  }
         |}
      """,
      project,
      errorCode = 2009,
      legacy = false
    )
  }
}
