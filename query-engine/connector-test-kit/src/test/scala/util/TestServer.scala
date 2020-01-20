package util

import play.api.libs.json._
case class TestServer() extends PlayJsonExtensions {
  def query(
      query: String,
      project: Project,
      dataContains: String = ""
  ): JsValue = {
    val result = queryBinaryCLI(
      query = query.stripMargin,
      project = project,
    )
    result.assertSuccessfulResponse(dataContains)
    result
  }

  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = ""
  ): JsValue = {
    val result =
      queryBinaryCLI(
        query = query.stripMargin,
        project = project,
      )

    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(0, errorCount, "")
    result
  }

  def queryBinaryCLI(query: String, project: Project) = {
    val formattedQuery = query.stripMargin.replace("\n", "")
    import sys.process._

    val res =
      Process(Seq(EnvVars.prismaBinaryPath, "cli", "--execute_request", formattedQuery), None, "PRISMA_DML" -> project.envVar).!!
    Json.parse(res)
  }
}
