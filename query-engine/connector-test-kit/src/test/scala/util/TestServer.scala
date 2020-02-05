package util

import play.api.libs.json._

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
    Json.obj("query" -> formattedQuery, "variables" -> Json.obj())
  }

  def createMultiQuery(queries: Array[String]): JsValue = {
    Json.obj("batch" -> queries.map(createSingleQuery))
  }

  def queryBinaryCLI(request: JsValue, project: Project) = {
    val encoded_query = UTF8Base64.encode(Json.stringify(request))

    val response = project.isPgBouncer match {
      case true =>
        Process(Seq(EnvVars.prismaBinaryPath, "--enable_raw_queries", "--always_force_transactions", "cli", "--execute_request", encoded_query),
                None,
                "PRISMA_DML" -> project.pgBouncerEnvVar).!!

      case false => {
        Process(Seq(EnvVars.prismaBinaryPath, "--enable_raw_queries", "cli", "--execute_request", encoded_query), None, "PRISMA_DML" -> project.envVar).!!
      }
    }
    val decoded_response = UTF8Base64.decode(response)
    println(decoded_response)
    Json.parse(decoded_response)
  }
}
