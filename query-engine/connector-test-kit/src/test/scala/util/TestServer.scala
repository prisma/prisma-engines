package util

import play.api.libs.json._

import scala.util.{Failure, Success, Try}
import scala.sys.process.Process

case class TestServer() extends PlayJsonExtensions {
  def query(
      query: String,
      project: Project,
      dataContains: String = "",
  ): JsValue = {
    val result = queryBinaryCLI(
      request = createSingleQuery(query),
      project = project,
    )
    result.assertSuccessfulResponse(dataContains)
    result
  }

  def batch(
      queries: Array[String],
      project: Project,
  ): JsValue = {
    val result = queryBinaryCLI(
      request = createMultiQuery(queries),
      project = project,
    )
    result
  }

  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = "",
  ): JsValue = {
    val result =
      queryBinaryCLI(
        request = createSingleQuery(query),
        project = project,
      )

    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(0, errorCount, "")
    result
  }

  def createSingleQuery(query: String): JsValue = {
    val formattedQuery = query.stripMargin.replace("\n", "")
    println(formattedQuery)
    Json.obj("query" -> formattedQuery, "variables" -> Json.obj())
  }

  def createMultiQuery(queries: Array[String]): JsValue = {
    Json.obj("batch" -> queries.map(createSingleQuery))
  }

  def queryBinaryCLI(request: JsValue, project: Project) = {
    val encoded_query = UTF8Base64.encode(Json.stringify(request))

    val response = project.isPgBouncer match {
      case true =>
        Process(
          Seq(
            EnvVars.prismaBinaryPath,
            "--enable_raw_queries",
            "--always_force_transactions",
            "cli",
            "--execute_request",
            encoded_query
          ),
          None,
          "PRISMA_DML" -> project.pgBouncerEnvVar
        ).!!

      case false => {
        Process(
          Seq(
            EnvVars.prismaBinaryPath,
            "--enable_raw_queries",
            "cli",
            "--execute_request",
            encoded_query
          ),
          None,
          "PRISMA_DML" -> project.envVar,
        ).!!
      }
    }
    val lines = response.linesIterator.toVector
    println(lines.mkString("\n"))

    val responseMarker = "Response: " // due to race conditions the response can not always be found in the last line
    val responseLine   = lines.find(_.startsWith(responseMarker)).get.stripPrefix(responseMarker).stripSuffix("\n")
    println(lines.mkString("\n"))

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
