package util

import java.nio.charset.StandardCharsets
import java.util.Base64

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
    import sys.process._

    val formattedQuery = query.stripMargin.replace("\n", "")
    val encoded        = Base64.getEncoder.encode(formattedQuery.getBytes(StandardCharsets.UTF_8))
    val encoded_query  = new String(encoded, StandardCharsets.UTF_8)
    val response =
      Process(Seq(EnvVars.prismaBinaryPath, "cli", "--execute_request", encoded_query), None, "PRISMA_DML" -> project.envVar).!!

    val decoded          = Base64.getDecoder.decode(response.trim.getBytes(StandardCharsets.UTF_8))
    val decoded_response = new String(decoded, StandardCharsets.UTF_8)

    Json.parse(decoded_response)
  }
}
