package util

import org.scalatest.{Suite, WordSpecLike}
import play.api.libs.json.JsValue
import util.ConnectorTag.{MongoConnectorTag, RelationalConnectorTag}
import wvlet.log.LogFormatter.{BareFormatter, SimpleLogFormatter}
import wvlet.log.{LogLevel, LogSupport, Logger}

case class TestDataModels(
    mongo: Vector[String],
    sql: Vector[String]
)

object TestDataModels {
  def apply(mongo: String, sql: String): TestDataModels = {
    TestDataModels(mongo = Vector(mongo), sql = Vector(sql))
  }
}

case class TestDataModelsWrapper(
    dataModel: TestDataModels,
    connectorTag: ConnectorTag,
    connectorName: String,
    database: TestDatabase
)(implicit suite: Suite)
    extends WordSpecLike
    with LogSupport {

  Logger.setDefaultFormatter(BareFormatter)
  Logger.setDefaultLogLevel(LogLevel.apply(sys.env.getOrElse("LOG_LEVEL", "debug").toLowerCase))

  def test[T](indexToTest: Int)(fn: String => T)     = internal(Some(indexToTest))(fn)
  def test[T](fn: String => T)                       = internal(None)(fn)
  def testV11[T](indexToTest: Int)(fn: Project => T) = internalV11(Some(indexToTest))(fn)
  def testV11[T](fn: Project => T)                   = internalV11(None)(fn)

  private def internalV11[T](indexToTest: Option[Int])(fn: Project => T) = {
    internal(indexToTest) { dm =>
      val project = ProjectDsl.fromString(dm)
      database.setup(project)
      fn(project)
    }
  }

  private def internal[T](indexToTest: Option[Int])(fn: String => T) = {
    val dataModelsToTest = connectorTag match {
      case MongoConnectorTag         => dataModel.mongo
      case _: RelationalConnectorTag => dataModel.sql
    }

    var didRunATest = false
    dataModelsToTest.zipWithIndex.foreach {
      case (dm, index) =>
        val testThisOne = indexToTest.forall(_ == index)
        if (testThisOne) {
          didRunATest = testThisOne

          debug("*" * 75)
          debug(s"name:  $connectorName")
          error(s"index: $index")
          debug(s"tag:   ${connectorTag.entryName}")
          debug(s"schema: \n $dm")
          debug("*" * 75)

          fn(dm)
        }
    }

    if (!didRunATest) {
      error("There was no Datamodel for the provided index!")
    }
  }

}

case class QueryParams(
    selection: String,
    where: (JsValue, String) => String,
    whereMulti: (JsValue, String) => Vector[String],
) {
  def whereFirst(json: JsValue, path: String): String = this.whereMulti(json, path)(0)
  def whereAll(json: JsValue, path: String): String   = "[" + this.whereMulti(json, path).mkString(", ") + "]"
}

case class TestAbstraction(datamodel: String, parent: QueryParams, child: QueryParams)

case class AbstractTestDataModels(
    mongo: Vector[TestAbstraction],
    sql: Vector[TestAbstraction]
)

object AbstractTestDataModels {
  def apply(mongo: TestAbstraction, sql: TestAbstraction): AbstractTestDataModels = {
    AbstractTestDataModels(mongo = Vector(mongo), sql = Vector(sql))
  }
}

case class AbstractTestDataModelsWrapper(
    dataModel: AbstractTestDataModels,
    connectorTag: ConnectorTag,
    connectorName: String,
    database: TestDatabase
)(implicit suite: Suite)
    extends WordSpecLike
    with LogSupport {

  Logger.setDefaultFormatter(BareFormatter)
  Logger.setDefaultLogLevel(LogLevel.apply(sys.env.getOrElse("LOG_LEVEL", "debug").toLowerCase))

  def test[T](indexToTest: Int)(fn: TestAbstraction => T) = internal(Some(indexToTest))(fn)
  def test[T](fn: TestAbstraction => T)                   = internal(None)(fn)
  def testV11[T](indexToTest: Int)(fn: Project => T)      = internalV11(Some(indexToTest))(fn)
  def testV11[T](fn: Project => T)                        = internalV11(None)(fn)

  private def internalV11[T](indexToTest: Option[Int])(fn: Project => T) = {
    internal(indexToTest) { dm =>
      val project = ProjectDsl.fromString(dm.datamodel)
      database.setup(project)
      fn(project)
    }
  }

  private def internal[T](indexToTest: Option[Int])(fn: TestAbstraction => T) = {
    val dataModelsToTest = connectorTag match {
      case MongoConnectorTag         => dataModel.mongo
      case _: RelationalConnectorTag => dataModel.sql
    }

    var didRunATest = false
    dataModelsToTest.zipWithIndex.foreach {
      case (dm, index) =>
        val testThisOne = indexToTest.forall(_ == index)
        if (testThisOne) {
          didRunATest = testThisOne

          debug("*" * 75)
          debug(s"name:  $connectorName")
          error(s"index: $index")
          debug(s"tag:   ${connectorTag.entryName}")
          debug(s"schema: \n ${dm.datamodel}")
          debug("*" * 75)

          fn(dm)
        }
    }

    if (!didRunATest) {
      error("There was no Datamodel for the provided index!")
    }
  }

}
