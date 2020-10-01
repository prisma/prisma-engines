package util

import enumeratum.{Enum, EnumEntry}
import org.scalatest.{Suite, SuiteMixin, Tag}

sealed trait AssociatedWithConnectorTags {
  def tag: ConnectorTag
}

object IgnorePostgres extends Tag("ignore.postgres") with AssociatedWithConnectorTags {
  override def tag = ConnectorTag.PostgresConnectorTag
}
object IgnoreMySql extends Tag("ignore.mysql") with AssociatedWithConnectorTags {
  override def tag = ConnectorTag.MySqlConnectorTag
}
object IgnoreMySql56 extends Tag("ignore.mysql56") with AssociatedWithConnectorTags {
  override def tag = ConnectorTag.Mysql56ConnectorTag
}
object IgnoreMongo extends Tag("ignore.mongo") with AssociatedWithConnectorTags {
  override def tag = ConnectorTag.MongoConnectorTag
}
object IgnoreSQLite extends Tag("ignore.sqlite") with AssociatedWithConnectorTags {
  override def tag = ConnectorTag.SQLiteConnectorTag
}
object IgnoreMsSql extends Tag("ignore.mssql") with AssociatedWithConnectorTags {
  override def tag = ConnectorTag.MsSqlConnectorTag
}

object IgnoreSet {
  val ignoreConnectorTags = Set(IgnorePostgres, IgnoreMySql, IgnoreMySql56, IgnoreMongo, IgnoreSQLite, IgnoreMsSql)

  def byName(name: String): Option[AssociatedWithConnectorTags] = ignoreConnectorTags.find(_.name == name)
}

sealed trait ConnectorTag extends EnumEntry {
  def superTags(): Seq[ConnectorTag] = Seq()
}

object ConnectorTag extends Enum[ConnectorTag] {
  def values = findValues

  sealed trait RelationalConnectorTag extends ConnectorTag

  object MySqlConnectorTag extends RelationalConnectorTag
  object Mysql56ConnectorTag extends RelationalConnectorTag {
    override def superTags() = Seq(MySqlConnectorTag)
  }

  object PostgresConnectorTag       extends RelationalConnectorTag
  object SQLiteConnectorTag         extends RelationalConnectorTag
  object MsSqlConnectorTag          extends RelationalConnectorTag
  sealed trait DocumentConnectorTag extends ConnectorTag
  object MongoConnectorTag          extends DocumentConnectorTag
}

trait ConnectorAwareTest extends SuiteMixin { self: Suite with ApiSpecBase =>
  import IgnoreSet._

  lazy val connectorConfig = ConnectorConfig.instance
  lazy val connector       = connectorConfig.provider

  lazy val connectorTag = connector match {
    case "mongo"                   => ConnectorTag.MongoConnectorTag
    case "mysql56"                 => ConnectorTag.Mysql56ConnectorTag
    case "mysql"                   => ConnectorTag.MySqlConnectorTag
    case "postgres" | "postgresql" => ConnectorTag.PostgresConnectorTag
    case "sqlite"                  => ConnectorTag.SQLiteConnectorTag
    case "sqlserver"               => ConnectorTag.MsSqlConnectorTag
  }

  def capabilities: ConnectorCapabilities               = connectorConfig.capabilities
  def runOnlyForConnectors: Set[ConnectorTag]           = ConnectorTag.values.toSet
  def doNotRunForConnectors: Set[ConnectorTag]          = Set.empty
  def runOnlyForCapabilities: Set[ConnectorCapability]  = Set.empty
  def doNotRunForCapabilities: Set[ConnectorCapability] = Set.empty
  def doNotRun: Boolean                                 = false

  abstract override def tags: Map[String, Set[String]] = { // this must NOT be a val. Otherwise ScalaTest does not behave correctly.
    if (shouldSuiteBeIgnored || doNotRun) {
      ignoreAllTests
    } else {
      ignoredTestsBasedOnIndividualTagging
    }
  }

  private lazy val shouldSuiteBeIgnored: Boolean = { // this must be a val. Otherwise printing would happen many times.
    val connectorHasTheRightCapabilities = runOnlyForCapabilities.forall(capabilities.has) || runOnlyForCapabilities.isEmpty
    val connectorHasAWrongCapability     = doNotRunForCapabilities.exists(capabilities.has)
    val isTheRightConnector              = runOnlyForConnectors.contains(connectorTag) && !doNotRunForConnectors.contains(connectorTag)

    if (!isTheRightConnector) {
      error(s"""the suite ${self.getClass.getSimpleName} will be ignored because the current connector is not right
           | allowed connectors: ${runOnlyForConnectors.mkString(",")}
           | disallowed connectors: ${doNotRunForConnectors.mkString(",")}
           | current connector: $connectorTag
         """.stripMargin)
      true
    } else if (!connectorHasTheRightCapabilities) {
      error(
        s"""the suite ${self.getClass.getSimpleName} will be ignored because it does not have the right capabilities
           | required capabilities: ${runOnlyForCapabilities.mkString(",")}
           | connector capabilities: ${capabilities.capabilities.mkString(",")}
         """.stripMargin
      )
      true
    } else if (connectorHasAWrongCapability) {
      error(
        s"""the suite ${self.getClass.getSimpleName} will be ignored because it has a wrong capability
           | wrong capabilities: ${doNotRunForCapabilities.mkString(",")}
           | connector capabilities: ${capabilities.capabilities.mkString(",")}
         """.stripMargin
      )
      true
    } else {
      false
    }
  }

  def ifConnectorIsNotSQLite[T](assertion: => T): Unit = if (connectorTag != ConnectorTag.SQLiteConnectorTag) assertion
  def ifConnectorIsSQLite[T](assertion: => T): Unit    = if (connectorTag == ConnectorTag.SQLiteConnectorTag) assertion
  def ifConnectorIsNotMongo[T](assertion: => T): Unit  = if (connectorTag != ConnectorTag.MongoConnectorTag) assertion
  def ifConnectorIsActive[T](assertion: => T): Unit = {
    // FIXME: check if we need can bring this back, discuss with do4gr
//    if (connector.active && connectorTag != ConnectorTag.MongoConnectorTag) assertion
  }

  private def ignoredTestsBasedOnIndividualTagging = {
    super.tags.mapValues { tagNames =>
      val connectorTagsToIgnore: Set[ConnectorTag] = for {
        tagName   <- tagNames
        ignoreTag <- IgnoreSet.byName(tagName)
      } yield ignoreTag.tag

      val isIgnored = connectorTagsToIgnore.contains(connectorTag) || connectorTag.superTags().exists(st => connectorTagsToIgnore.contains(st))
      if (isIgnored) {
        tagNames ++ Set("org.scalatest.Ignore")
      } else {
        tagNames
      }
    }
  }

  private def ignoreAllTests = {
    testNames.map { testName =>
      testName -> Set("org.scalatest.Ignore")
    }.toMap
  }
}
