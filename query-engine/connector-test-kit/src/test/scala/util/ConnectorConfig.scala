package util

case class ConnectorConfig(
    provider: String,
    url: String
) {
  def capabilities = {
    provider match {
      case "sqlite"     => ConnectorCapabilities.sqlite
      case "postgresql" => ConnectorCapabilities.postgres
      case "mysql"      => ConnectorCapabilities.mysql
    }
  }
}

object ConnectorConfig {
  lazy val instance: ConnectorConfig = {
    val filePath        = EnvVars.serverRoot + "/current_connector"
    val connectorToTest = scala.io.Source.fromFile(filePath).mkString.lines.next().trim

    connectorToTest match {
      case "sqlite"                  => ConnectorConfig("sqlite", "file://$DB_FILE")
      case "postgres" | "postgresql" => ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgresHost:5432/db?schema=$$DB&connection_limit=1")

      case "mysql8"                   => ConnectorConfig("mysql", s"mysql://root:prisma@$mysql_8_0_Host:$mysql_8_0_Port/$$DB?connection_limit=1&ssl-mode=disabled&sslaccept=accept_invalid_certs")
      case x                         => sys.error(s"Connector $x is not supported yet.")
    }
  }

  lazy val postgresHost = {
    if (EnvVars.isBuildkite) {
      "test-db-postgres"
    } else {
      "127.0.0.1"
    }
  }

  lazy val mysql_5_7_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-mysql-5-7"
    } else {
      "127.0.0.1"
    }
  }

  lazy val mysql_8_0_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-mysql-8-0"
    } else {
      "127.0.0.1"
    }
  }

  lazy val mysql_8_0_Port = {
    if (EnvVars.isBuildkite) {
      3306
    } else {
      3307
    }
  }
}
