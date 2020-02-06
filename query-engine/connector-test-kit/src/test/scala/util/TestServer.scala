package util

import play.api.libs.json._

import scala.util.{Failure, Success, Try}
case class TestServer() extends PlayJsonExtensions {
  def query(
      query: String,
      project: Project,
      dataContains: String = ""
  ): JsValue = {
    val result = queryBinaryCLI(
      query = query,
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
        query = query,
        project = project,
      )

    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(0, errorCount, "")
    result
  }

  def queryBinaryCLI(query: String, project: Project) = {
    import sys.process._

    val formattedQuery = query.stripMargin.replace("\n", "")
    println(formattedQuery)
    val encoded_query = UTF8Base64.encode(formattedQuery)
    val response: String = project.isPgBouncer match {
      case true =>
        Process(Seq(EnvVars.prismaBinaryPath, "--always_force_transactions", "cli", "--execute_request", encoded_query),
                None,
                "PRISMA_DML" -> project.pgBouncerEnvVar).!!
      case false => Process(Seq(EnvVars.prismaBinaryPath, "cli", "--execute_request", encoded_query), None, "PRISMA_DML" -> project.envVar).!!
    }
    val lines          = response.linesIterator.toVector
    val responseMarker = "Response: " // due to race conditions the response can not always be found in the last line
    val responseLine   = lines.find(_.startsWith(responseMarker)).get.stripPrefix(responseMarker).stripSuffix("\n")

    println(response)
    Try(UTF8Base64.decode(responseLine)) match {
      case Success(decodedResponse) =>
        println(decodedResponse)
        Json.parse(decodedResponse)

      case Failure(e) =>
        println(s"Error while decoding this line: \n$responseLine")
        throw e
    }

  }
}
