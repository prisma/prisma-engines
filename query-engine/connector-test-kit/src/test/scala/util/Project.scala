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

    val provider = config.provider.stripSuffix("56") match {
      case "vitess" => "mysql"
      case provider => provider
    }

    s"""
           |datasource test {
           |  provider = "${provider}"
           |  url = "$dataSourceUrl"
           |}
    """.stripMargin
  }

  val generatorBlock: String = {
    s"""
       |generator client {
       |  provider = "prisma-client-js"
       |  previewFeatures = ["mongodb", "orderByRelation", "napi", "selectRelationCount", "orderByAggregateGroup"]
       |}
    """.stripMargin
  }

  val fullDatamodel = {
    dataSourceConfig + "\n" + generatorBlock + "\n" + dataModel
  }

  val fullDatamodelBase64Encoded = UTF8Base64.encode(fullDatamodel)

  val dataModelPath: String = {
    val pathName = s"${EnvVars.serverRoot}/db/$id.prisma"
    val file     = new File(pathName)
    val writer   = new PrintWriter(file)

    try {
      fullDatamodel.foreach(writer.print)
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
