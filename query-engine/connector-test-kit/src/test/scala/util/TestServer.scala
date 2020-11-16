package util

import play.api.libs.json._
import wvlet.log.LogFormatter.SimpleLogFormatter
import wvlet.log.{LogLevel, LogSupport, Logger}

import scala.sys.process.Process
import scala.util.{Failure, Success, Try}

case class TestServer() extends PlayJsonExtensions with LogSupport {
  val logLevel = sys.env.getOrElse("LOG_LEVEL", "debug").toLowerCase

  Logger.setDefaultFormatter(SimpleLogFormatter)
  Logger.setDefaultLogLevel(LogLevel.apply(logLevel))

  // The standard query test method using projects.
  def query(
      query: String,
      project: Project,
      dataContains: String = "",
      legacy: Boolean = true,
      batchSize: Int = 5000,
  ): JsValue = {
    val result = queryBinary(
      request = createSingleQuery(query),
      encodedDataModel = project.fullDatamodelBase64Encoded,
      legacy = legacy,
      batchSize = batchSize,
    )

    result._1.assertSuccessfulResponse(dataContains)
    result._1
  }

  // The standard query test method for failures, using projects.
  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = "",
      legacy: Boolean = true,
      // Assertions of the form (jsonPath, expectedValue).
      errorMetaContains: Array[(String, String)] = Array.empty,
  ): JsValue = {
    val result =
      queryBinary(
        request = createSingleQuery(query),
        encodedDataModel = project.fullDatamodelBase64Encoded,
        legacy = legacy,
      )

    result._1.assertFailingResponse(errorCode, errorCount, errorContains, errorMetaContains)
    result._1
  }

  // Only for testing low level queries, usually not required.
  def query_with_logged_requests(
      query: String,
      project: Project,
  ): (JsValue, Vector[String]) = {
    val result = queryBinary(
      request = createSingleQuery(query),
      encodedDataModel = project.fullDatamodelBase64Encoded,
      log_requests = true
    )

    result._1.assertSuccessfulResponse()
    result
  }

  // Standard method for testing batch requests.
  def batch(
      queries: Seq[String],
      transaction: Boolean,
      project: Project,
      legacy: Boolean = true,
  ): JsValue = {
    val result = queryBinary(
      request = createMultiQuery(queries, transaction),
      encodedDataModel = project.fullDatamodelBase64Encoded,
      legacy = legacy,
    )
    result._1
  }

  // Query without the intermediate utility of `Project`, which means that the caller
  // is responsible to provide a full data model with generator and data source if the
  // call is to be successful.
  // Return value is the raw request. Caller has to do assertions.
  def queryDirect(
      query: String,
      datamodel: String,
      env_overrides: Map[String, String] = Map()
  ): JsValue = {
    val result = queryBinary(
      request = createSingleQuery(query),
      encodedDataModel = UTF8Base64.encode(datamodel),
      legacy = false,
      env_overrides = env_overrides,
    )

    result._1
  }

  private def createSingleQuery(query: String): JsValue = {
    val formattedQuery = query.stripMargin.replace("\n", "")
    debug(formattedQuery)
    Json.obj("query" -> formattedQuery, "variables" -> Json.obj())
  }

  private def createMultiQuery(queries: Seq[String], transaction: Boolean): JsValue = {
    Json.obj("batch" -> queries.map(createSingleQuery), "transaction" -> transaction)
  }

  // Fires a one-off query against the query engine binary, using the CLI mode instead of the server mode.
  private def queryBinary(
      request: JsValue,
      encodedDataModel: String, // Base64 encoded full data model string
      legacy: Boolean = true,
      batchSize: Int = 5000,
      log_requests: Boolean = false,
      env_overrides: Map[String, String] = Map() // Overrides existing env keys, else additive.
  ): (JsValue, Vector[String]) = {
    val encoded_query    = UTF8Base64.encode(Json.stringify(request))
    val binaryLogLevel   = "RUST_LOG" -> s"query_engine=$logLevel,quaint=$logLevel,query_core=$logLevel,query_connector=$logLevel,sql_query_connector=$logLevel,prisma_models=$logLevel,sql_introspection_connector=$logLevel"
    val log_requests_env = if (log_requests) { "LOG_QUERIES" -> "y" } else { ("", "") }

    val params = legacy match {
      case true =>
        Seq(
          EnvVars.prismaBinaryPath,
          "--enable-experimental=all",
          "--enable-raw-queries",
          "--datamodel",
          encodedDataModel,
          "cli",
          "execute-request",
          "--legacy",
          encoded_query
        )

      case false =>
        Seq(
          EnvVars.prismaBinaryPath,
          "--enable-experimental=all",
          "--enable-raw-queries",
          "--datamodel",
          encodedDataModel,
          "cli",
          "execute-request",
          encoded_query
        )
    }

    val env = Seq(
      "QUERY_BATCH_SIZE" -> batchSize.toString,
      binaryLogLevel,
      log_requests_env,
    )

    val process = if (EnvVars.isWindows) {
      Process(params)
    } else {
      Process(
        params,
        None,
        env ++ env_overrides.toSeq: _*
      )
    }

    val response = process.!!
    val lines    = response.linesIterator.toVector
    debug(lines.mkString("\n"))

    val responseMarker = "Response: " // due to race conditions the response can not always be found in the last line
    val responseLine   = lines.find(_.startsWith(responseMarker)).get.stripPrefix(responseMarker).stripSuffix("\n")

    Try(UTF8Base64.decode(responseLine)) match {
      case Success(decodedResponse) =>
        debug(decodedResponse)
        (Json.parse(decodedResponse), lines)

      case Failure(e) =>
        error(s"Error while decoding this line: \n$responseLine")
        throw e
    }
  }
}
