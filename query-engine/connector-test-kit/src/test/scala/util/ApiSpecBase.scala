package util

import org.scalatest.{BeforeAndAfterAll, BeforeAndAfterEach, Suite}
import play.api.libs.json.JsString
import util.ConnectorCapability.RelationLinkListCapability
import wvlet.log.LogFormatter.SimpleLogFormatter
import wvlet.log.{LogLevel, LogSupport, Logger}

import scala.concurrent.ExecutionContext

trait ApiSpecBase extends ConnectorAwareTest with BeforeAndAfterEach with BeforeAndAfterAll with PlayJsonExtensions with StringMatchers with LogSupport {
  self: Suite =>

  Logger.setDefaultFormatter(SimpleLogFormatter)
  Logger.setDefaultLogLevel(LogLevel.apply(sys.env.getOrElse("LOG_LEVEL", "debug").toLowerCase))

  implicit val ec                 = ExecutionContext.global
  implicit lazy val implicitSuite = self
  val server                      = TestServer()
  val database                    = TestDatabase()

  override protected def beforeAll(): Unit = {
    error(s">>> Starting ${this.getClass.getSimpleName}")
    super.beforeAll()
    PrismaRsBuild()
  }

  def escapeString(str: String) = JsString(str).toString()

  implicit def testDataModelsWrapper(testDataModel: TestDataModels): TestDataModelsWrapper = {
    TestDataModelsWrapper(testDataModel, connectorTag, connector, database)
  }

  implicit def abstractTestDataModelsWrapper(testDataModel: AbstractTestDataModels): AbstractTestDataModelsWrapper = {
    AbstractTestDataModelsWrapper(testDataModel, connectorTag, connector, database)
  }

  val listInlineArgument = if (capabilities.has(RelationLinkListCapability)) {
    "references: [id]"
  } else {
    ""
  }

  val relationInlineAttribute = if (capabilities.has(RelationLinkListCapability)) {
    s"@relation($listInlineArgument)"
  } else {
    ""
  }
}
