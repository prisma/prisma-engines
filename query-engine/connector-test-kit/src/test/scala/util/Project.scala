package util

import java.io.{File, PrintWriter}
import java.nio.charset.StandardCharsets
import java.util.Base64

import org.scalatest.Suite

case class Project(
    id: String,
    dataModel: String,
) {
  val dataSourceUrl: String = {
    ConnectorConfig.instance.url
      .replaceAllLiterally("$DB_FILE", s"${EnvVars.serverRoot}/db/$id.db")
      .replaceAllLiterally("$DB", id)
  }

  val dataSourceConfig: String = {
    val config = ConnectorConfig.instance

    s"""
           |datasource test {
           |  provider = "${config.provider}"
           |  url = "${dataSourceUrl}"
           |}
    """.stripMargin
  }

  val dataModelWithDataSourceConfig = {
    dataSourceConfig + "\n" + dataModel
  }

  val envVar = UTF8Base64.encode(dataModelWithDataSourceConfig)

  val pgBouncerEnvVar = {
    val host = {
      if (EnvVars.isBuildkite) {
        "test-db-pgbouncer"
      } else {
        "127.0.0.1"
      }
    }

    val url = s"postgresql://postgres:prisma@$host:6432/db?schema=$id&connection_limit=1"

    val config =
      s"""
         |datasource test {
         |  provider = "${ConnectorConfig.instance.provider}"
         |  url = "${url}"
         |}
         |
         |$dataModel
      """.stripMargin

    UTF8Base64.encode(config)
  }

  val isPgBouncer = ConnectorConfig.instance.isBouncer

  val dataModelPath: String = {
    val pathName = s"${EnvVars.serverRoot}/db/$id.prisma"
    val file     = new File(pathName)
    val writer   = new PrintWriter(file)

    try {
      dataModelWithDataSourceConfig.foreach(writer.print)
    } finally {
      writer.close()
    }

    pathName
  }
}

trait Dsl {
  val testProjectId = "default@default"

  def fromStringWithId(id: String)(sdlString: String): Project = {
    Project(id = id, dataModel = sdlString)
  }

  def fromString(sdlString: String)(implicit suite: Suite): Project = {
    Project(id = projectId(suite), dataModel = sdlString.stripMargin)
  }

  // this exists only for backwards compatibility to ease test conversion
  def fromStringV11()(sdlString: String)(implicit suite: Suite): Project = {
    fromString(sdlString)
  }

  private def projectId(suite: Suite): String = {
    suite.getClass.getSimpleName
  }
}

object ProjectDsl extends Dsl
object SchemaDsl  extends Dsl // this exists only for backwards compatibility to ease test conversion
