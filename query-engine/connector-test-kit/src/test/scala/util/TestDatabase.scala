package util

case class TestDatabase() {
  def setup(project: Project): Unit = {
    val engine = MigrationEngine(project)

    engine.resetAndSetupDatabase()
  }

  def truncateProjectTables(project: Project): Unit = {
    // FIXME: implement truncation instead of this stupid approach
    setup(project)
  }
}

case class MigrationEngine(project: Project) {

  val migrationId = "test_migration_id"
  val logLevel    = "RUST_LOG" -> sys.env.getOrElse("LOG_LEVEL", "debug").toLowerCase

  def resetAndSetupDatabase(): Unit = {
    import scala.sys.process._
    val config = ConnectorConfig.instance

    val flags = config.provider match {
      case "vitess" => "database_creation_not_allowed"
      case _ => ""
    }

    val cmd = List(EnvVars.migrationEngineBinaryPath, "cli", "-f", flags, "-d", project.fullDatamodelBase64Encoded, "qe-setup")

    cmd.!
  }
}
