package util

import scala.util.Try

case class ConnectorConfig(
    provider: String,
    url: String,
    isBouncer: Boolean,
    name: String,
) {
  def capabilities = {
    provider match {
      case "sqlite"     => ConnectorCapabilities.sqlite
      case "postgresql" => ConnectorCapabilities.postgres
      case "mysql"      => ConnectorCapabilities.mysql
      case "sqlserver"  => ConnectorCapabilities.mssql
    }
  }
}

object ConnectorConfig {
  lazy val instance: ConnectorConfig = {
    val filePath = EnvVars.serverRoot + "/current_connector"
    val connectorToTest = Try {
      scala.io.Source.fromFile(filePath).mkString.lines.next().trim
    }.getOrElse(sys.env.getOrElse("TEST_CONNECTOR",
                                  sys.error("Neither current_connector file nor TEST_CONNECTOR found to decide which connector to test with. Aborting.")))

    connectorToTest match {
      case "sqlite" => ConnectorConfig("sqlite", "file://$DB_FILE", false, "sqlite")
      case "postgres9" | "postgresql9" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_9_Host:$postgres_9_Port/db?schema=$$DB&connection_limit=1", false, "postgres9")
      case "postgres10" | "postgresql10" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_10_Host:$postgres_10_Port/db?schema=$$DB&connection_limit=1", false, "postgres10")
      case "postgres11" | "postgresql11" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_11_Host:$postgres_11_Port/db?schema=$$DB&connection_limit=1", false, "postgres11")
      case "postgres12" | "postgresql12" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_12_Host:$postgres_12_Port/db?schema=$$DB&connection_limit=1", false, "postgres12")
      case "postgres13" | "postgresql13" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_13_Host:$postgres_13_Port/db?schema=$$DB&connection_limit=1", false, "postgres13")
      case "pgbouncer" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_11_Host:$postgres_11_Port/db?schema=$$DB&connection_limit=1", true, "pgbouncer")
      case "mysql"   => ConnectorConfig("mysql", s"mysql://root:prisma@$mysql_5_7_Host:3306/$$DB?connection_limit=1", false, "mysql")
      case "mysql8"  => ConnectorConfig("mysql", s"mysql://root:prisma@$mysql_8_0_Host:$mysql_8_0_Port/$$DB?connection_limit=1", false, "mysql8")
      case "mysql56"  => ConnectorConfig("mysql", s"mysql://root:prisma@$mysql_5_6_Host:$mysql_5_6_Port/$$DB?connection_limit=1", false, "mysql56")
      case "mariadb" => ConnectorConfig("mysql", s"mysql://root:prisma@$mariadb_Host:$mariadb_Port/$$DB?connection_limit=1", false, "mariadb")
      case "mssql2017" => ConnectorConfig("sqlserver", s"sqlserver://$mssql_2017_Host:$mssql_2017_Port;database=master;schema=$$DB;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED;encrypt=DANGER_PLAINTEXT", false, "mssql2017")
      case "mssql2019" => ConnectorConfig("sqlserver", s"sqlserver://$mssql_2019_Host:$mssql_2019_Port;database=master;schema=$$DB;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED;encrypt=DANGER_PLAINTEXT", false, "mssql2019")
      case x         => sys.error(s"Connector $x is not supported yet.")
    }
  }

  lazy val mssql_2019_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-mssql-2019"
    } else {
      "127.0.0.1"
    }
  }

  lazy val mssql_2017_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-mssql-2017"
    } else {
      "127.0.0.1"
    }
  }

  lazy val mssql_2019_Port = {
    1433
  }

  lazy val mssql_2017_Port = {
    if (EnvVars.isBuildkite) {
      1433
    } else {
      1434
    }
  }

  lazy val postgres_9_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-postgres-9"
    } else {
      "127.0.0.1"
    }
  }

  lazy val postgres_9_Port = {
    if (EnvVars.isBuildkite) {
      5432
    } else {
      5431
    }
  }

  lazy val postgres_10_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-postgres-10"
    } else {
      "127.0.0.1"
    }
  }

  lazy val postgres_10_Port = {
    5432
  }

  lazy val postgres_11_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-postgres-11"
    } else {
      "127.0.0.1"
    }
  }

  lazy val postgres_11_Port = {
    if (EnvVars.isBuildkite) {
      5432
    } else {
      5433
    }
  }

  lazy val postgres_12_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-postgres-12"
    } else {
      "127.0.0.1"
    }
  }

  lazy val postgres_12_Port = {
    if (EnvVars.isBuildkite) {
      5432
    } else {
      5434
    }
  }

  lazy val postgres_13_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-postgres-13"
    } else {
      "127.0.0.1"
    }
  }

  lazy val postgres_13_Port = {
    if (EnvVars.isBuildkite) {
      5432
    } else {
      5435
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

  lazy val mysql_5_6_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-mysql-5-6"
    } else {
      "127.0.0.1"
    }
  }


  lazy val mariadb_Host = {
    if (EnvVars.isBuildkite) {
      "test-db-mariadb"
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

  lazy val mysql_5_6_Port = {
    if (EnvVars.isBuildkite) {
      3306
    } else {
      3309
    }
  }

  lazy val mariadb_Port = {
    if (EnvVars.isBuildkite) {
      3306
    } else {
      3308
    }
  }
}
