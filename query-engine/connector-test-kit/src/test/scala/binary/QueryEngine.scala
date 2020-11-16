package binary

import org.scalatest.{FlatSpec, Matchers}
import util._

class QueryEngine extends FlatSpec with Matchers with ApiSpecBase {
  "Setting a data source override via env" should "prevent env errors" in {
    val config = ConnectorConfig.instance

    val header = s"""
        |datasource test {
        |  provider = "${config.provider.stripSuffix("56")}"
        |  url = env("NON_EXISTENT_KEY")
        |}
        |
        |generator js {
        |  provider = "prisma-client-js"
        |}
        """.stripMargin

    val datamodel =
      s"""
        |model A {
        |  id Int @id
        |}
      """.stripMargin

    // Used for datamodel string utilities to allow running in context of the current connector.
    val project = ProjectDsl.fromString(datamodel)
    database.setup(project)

    val result = server.queryDirect(
      """
          |mutation {
          |  createOneA(data: { id: 1 }) { id }
          |}
        """.stripMargin,
      header + "\n" + datamodel, // With invalid url
      env_overrides = Map("OVERWRITE_DATASOURCES" -> s"""[{"name": "test", "url": "${project.dataSourceUrl}"}]""") // Inject valid url
    )

    result.assertSuccessfulResponse()
    result.toString() should be("""{"data":{"createOneA":{"id":1}}}""")
  }
}
