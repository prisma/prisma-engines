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

  def query(
      query: String,
      project: Project,
      dataContains: String = "",
      legacy: Boolean = true,
      batchSize: Int = 5000,
  ): JsValue = {
    val result = queryBinaryCLI(
      request = createSingleQuery(query),
      project = project,
      legacy = legacy,
      batchSize = batchSize,
    )
    result.assertSuccessfulResponse(dataContains)
    result
  }

  def batch(
      queries: Array[String],
      project: Project,
      legacy: Boolean = true,
  ): JsValue = {
    val result = queryBinaryCLI(
      request = createMultiQuery(queries),
      project = project,
      legacy = legacy,
    )
    result
  }

  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = "",
      legacy: Boolean = true,
  ): JsValue = {
    val result =
      queryBinaryCLI(
        request = createSingleQuery(query),
        project = project,
        legacy = legacy,
      )

    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(errorCode, errorCount, errorContains)
    result
  }

  def createSingleQuery(query: String): JsValue = {
    val formattedQuery = query.stripMargin.replace("\n", "")
    debug(formattedQuery)
    Json.obj("query" -> formattedQuery, "variables" -> Json.obj())
  }

  def createMultiQuery(queries: Array[String]): JsValue = {
    Json.obj("batch" -> queries.map(createSingleQuery))
  }

  def queryBinaryCLI(request: JsValue, project: Project, legacy: Boolean = true, batchSize: Int = 5000) = {
    val encoded_query  = UTF8Base64.encode(Json.stringify(request))
    val binaryLogLevel = "RUST_LOG" -> s"prisma=$logLevel,quaint=$logLevel,query_core=$logLevel,query_connector=$logLevel,sql_query_connector=$logLevel,prisma_models=$logLevel,sql_introspection_connector=$logLevel"

    val response = (project.isPgBouncer, legacy) match {
      case (true, true) =>
        Process(
          Seq(
            EnvVars.prismaBinaryPath,
            "--enable-raw-queries",
            "--always-force-transactions",
            "cli",
            "execute-request",
            "--legacy",
            encoded_query
          ),
          None,
          "PRISMA_DML" -> project.pgBouncerEnvVar,
          "QUERY_BATCH_SIZE" -> batchSize.toString,
          binaryLogLevel,
        ).!!

      case (true, false) =>
        Process(
          Seq(
            EnvVars.prismaBinaryPath,
            "--enable-raw-queries",
            "--always-force-transactions",
            "cli",
            "execute-request",
            encoded_query
          ),
          None,
          "PRISMA_DML" -> project.pgBouncerEnvVar,
          "QUERY_BATCH_SIZE" -> batchSize.toString,
          binaryLogLevel,
        ).!!

      case (false, true) =>
        Process(
          Seq(
            EnvVars.prismaBinaryPath,
            "--enable-raw-queries",
            "cli",
            "execute-request",
            "--legacy",
            encoded_query
          ),
          None,
          "PRISMA_DML" -> project.envVar,
          "QUERY_BATCH_SIZE" -> batchSize.toString,
          binaryLogLevel,
        ).!!

      case (false, false) =>
        Process(
          Seq(
            EnvVars.prismaBinaryPath,
            "--enable-raw-queries",
            "cli",
            "execute-request",
            encoded_query
          ),
          None,
          "PRISMA_DML" -> project.envVar,
          "QUERY_BATCH_SIZE" -> batchSize.toString,
          binaryLogLevel,
        ).!!
    }

    val lines = response.linesIterator.toVector
    debug(lines.mkString("\n"))

    val responseMarker = "Response: " // due to race conditions the response can not always be found in the last line
    val responseLine   = lines.find(_.startsWith(responseMarker)).get.stripPrefix(responseMarker).stripSuffix("\n")
    debug(lines.mkString("\n"))

    Try(UTF8Base64.decode(responseLine)) match {
      case Success(decodedResponse) =>
        debug(decodedResponse)
        Json.parse(decodedResponse)

      case Failure(e) =>
        error(s"Error while decoding this line: \n$responseLine")
        throw e
    }
  }
}
