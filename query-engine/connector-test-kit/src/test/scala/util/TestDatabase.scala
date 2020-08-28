package util

import java.io.ByteArrayInputStream

import play.api.libs.json._

case class TestDatabase() {
  def setup(project: Project): Unit = {
    val engine = MigrationEngine(project)
    engine.setupDatabase()
    engine.schemaPush()
  }

  def truncateProjectTables(project: Project): Unit = {
    // FIXME: implement truncation instead of this stupid approach
    setup(project)
  }
}

case class MigrationEngine(project: Project) {
  implicit val schemaPushInputWrites  = Json.writes[SchemaPushInput]
  implicit val schemaPushOutputReads  = Json.reads[SchemaPushOutput]
  implicit val rpcResultReads         = Json.reads[RpcResult]

  val migrationId = "test_migration_id"
  val logLevel    = "RUST_LOG" -> sys.env.getOrElse("LOG_LEVEL", "debug").toLowerCase
//  val binaryLogLevel = "RUST_LOG" -> s"prisma=$logLevel,quaint=$logLevel,query_core=$logLevel,query_connector=$logLevel,sql_query_connector=$logLevel,prisma_models=$logLevel,sql_introspection_connector=$logLevel"

  def schemaPush(): Unit = {
    val input = SchemaPushInput(
      schema = project.dataModelWithDataSourceConfig,
      force = true,
    )

    val _: JsValue = sendRpcCall[SchemaPushInput, JsValue]("schemaPush", input)
  }

  def setupDatabase(): Unit = {
    import scala.sys.process._
    val cmd = List(EnvVars.migrationEngineBinaryPath, "cli", "-d", project.dataSourceUrl, "qe-setup")

    cmd.!
  }

  private def sendRpcCall[A, B](method: String, params: A)(implicit writes: OWrites[A], reads: Reads[B]): B = {
    sendRpcCallInternal[B](method, Json.toJsObject(params))
  }

  private def sendRpcCallInternal[B](method: String, params: JsObject)(implicit reads: Reads[B]): B = {
    val rpcCall = envelope(method, params)
//    println(s"sending to MigrationEngine: $rpcCall")

    val inputStream = new ByteArrayInputStream(rpcCall.toString.getBytes("UTF-8"))
    val output: String = {
      import scala.sys.process._

      (Process(
        Seq(
          EnvVars.migrationEngineBinaryPath,
          "-s",
          "-d",
          project.dataModelPath
        ),
        None,
        logLevel,
      ) #< inputStream).!!
    }

    val lastLine = output.lines.foldLeft("")((_, line) => line)
    Json.parse(lastLine).validate[RpcResult] match {
      case JsSuccess(rpcResult, _) => rpcResult.result.as[B]
      case e: JsError => {
        println(s"MigrationEngine responded: $output")
        sys.error(e.toString)
      }
    }
  }

  private def envelope(method: String, params: JsObject): JsValue = {
    val finalParams = params ++ Json.obj("sourceConfig" -> project.dataSourceConfig)
    Json.obj(
      "id"      -> "1",
      "jsonrpc" -> "2.0",
      "method"  -> method,
      "params"  -> finalParams
    )
  }
}

case class SchemaPushInput(
  schema: String,
  force: Boolean,
)

case class SchemaPushOutput(
    executedSteps: Int,
    warnings: Vector[String],
    unexecutable: Vector[String],
)

case class RpcResult(
    id: String,
    result: JsValue
)
