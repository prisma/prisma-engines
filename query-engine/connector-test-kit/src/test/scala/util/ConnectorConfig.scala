package util

import java.nio.charset.Charset
import java.nio.file.{Files, Paths}

import scala.util.Try

case class ConnectorConfig(
    provider: String,
    url: String,
    name: String,
) {
  def capabilities = {
    provider match {
      case "sqlite"     => ConnectorCapabilities.sqlite
      case "postgresql" => ConnectorCapabilities.postgres
      case "mysql"      => ConnectorCapabilities.mysql
      case "mysql56"    => ConnectorCapabilities.mysql
      case "vitess"     => ConnectorCapabilities.mysql
      case "sqlserver"  => ConnectorCapabilities.mssql
      case "mongodb"    => ConnectorCapabilities.mongo
    }
  }
}

object ConnectorConfig {
  lazy val instance: ConnectorConfig = {
    val filePath = EnvVars.serverRoot + "/current_connector"

    val connectorToTest = Try {
      new String(Files.readAllBytes(Paths.get(filePath)), Charset.defaultCharset()).trim
    }.getOrElse(sys.env.getOrElse("TEST_CONNECTOR",
                                  sys.error("Neither current_connector file nor TEST_CONNECTOR found to decide which connector to test with. Aborting.")))

    connectorToTest match {
      case "sqlite" => ConnectorConfig("sqlite", "file://$DB_FILE", "sqlite")

      case "postgres9" | "postgresql9" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_9_Host:$postgres_9_Port/db?schema=$$DB&connection_limit=1", "postgres9")

      case "postgres10" | "postgresql10" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_10_Host:$postgres_10_Port/db?schema=$$DB&connection_limit=1", "postgres10")

      case "postgres11" | "postgresql11" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_11_Host:$postgres_11_Port/db?schema=$$DB&connection_limit=1", "postgres11")

      case "postgres12" | "postgresql12" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_12_Host:$postgres_12_Port/db?schema=$$DB&connection_limit=1", "postgres12")

      case "postgres13" | "postgresql13" =>
        ConnectorConfig("postgresql", s"postgresql://postgres:prisma@$postgres_13_Host:$postgres_13_Port/db?schema=$$DB&connection_limit=1", "postgres13")

      case "pgbouncer" =>
        ConnectorConfig("postgresql",
                        s"postgresql://postgres:prisma@$pgbouncer_host:$pgbouncer_port/db?schema=$$DB&connection_limit=1&pgbouncer=true",
                        "pgbouncer")

      case "mysql"   => ConnectorConfig("mysql", s"mysql://root:prisma@$mysql_5_7_Host:3306/$$DB?connection_limit=1", "mysql")
      case "mysql8"  => ConnectorConfig("mysql", s"mysql://root:prisma@$mysql_8_0_Host:$mysql_8_0_Port/$$DB?connection_limit=1", "mysql8")
      case "mysql56" => ConnectorConfig("mysql56", s"mysql://root:prisma@$mysql_5_6_Host:$mysql_5_6_Port/$$DB?connection_limit=1", "mysql56")
      case "mariadb" => ConnectorConfig("mysql", s"mysql://root:prisma@$mariadb_Host:$mariadb_Port/$$DB?connection_limit=1", "mariadb")

      case "vitess_5_7" => ConnectorConfig("vitess", s"mysql://root:prisma@127.0.0.1:33577/$$DB?connection_limit=1", "vitess")
      case "vitess_8_0" => ConnectorConfig("vitess", s"mysql://root:prisma@127.0.0.1:33807/$$DB?connection_limit=1", "vitess")

      case "mssql2017" =>
        ConnectorConfig(
          "sqlserver",
          s"sqlserver://$mssql_2017_Host:$mssql_2017_Port;database=master;schema=$$DB;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED",
          "mssql2017"
        )

      case "mssql2019" =>
        ConnectorConfig(
          "sqlserver",
          s"sqlserver://$mssql_2019_Host:$mssql_2019_Port;database=master;schema=$$DB;user=SA;password=<YourStrong@Passw0rd>;trustServerCertificate=true;isolationLevel=READ UNCOMMITTED",
          "mssql2019"
        )

      case "mongodb" => ConnectorConfig("mongodb", s"mongodb://prisma:prisma@$mongo_host:$mongo_port/$$DB?authSource=admin", "mongodb")
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

  lazy val mongo_host = {
    if (EnvVars.isBuildkite) {
      "test-db-mongo-4"
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

  lazy val mongo_port = {
    if (EnvVars.isBuildkite) {
      27017
    } else {
      27017
    }
  }

  lazy val pgbouncer_host = {
    if (EnvVars.isBuildkite) {
      "test-db-pgbouncer"
    } else {
      "127.0.0.1"
    }
  }

  lazy val pgbouncer_port = {
    if (EnvVars.isBuildkite) {
      6432
    } else {
      6432
    }
  }
}
