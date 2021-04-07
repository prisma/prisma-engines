import sbt.Keys._
import Dependencies._

import java.nio.charset.Charset
import java.nio.file.{Files, Paths}
import scala.util.Try

name := "connector-test-kit"

def allowParallelExecution(): Boolean = {
  val filePath = sys.env.get("SERVER_ROOT").get.toString + "/current_connector"

  val connectorToTest = Try {
    new String(Files.readAllBytes(Paths.get(filePath)), Charset.defaultCharset()).trim
  }.getOrElse(sys.env.getOrElse("TEST_CONNECTOR",
    sys.error("Neither current_connector file nor TEST_CONNECTOR found to decide which connector to test with. Aborting.")))

  connectorToTest match {
    case "pgbouncer" => false
    case _ => true
  }
}

lazy val commonSettings = Seq(
  organization := "com.prisma",
  organizationName := "Prisma",
  scalaVersion := "2.12.7",
  parallelExecution in Test := allowParallelExecution(),
  publishArtifact in (Test, packageDoc) := false,
  publishArtifact in (Compile, packageDoc) := false,
  publishArtifact in packageDoc := false,
  publishArtifact in packageSrc := false,
  sources in (Compile,doc) := Seq.empty, // todo Somehow, after all these settings, there's STILL API docs getting generated somewhere.
  // We should gradually introduce https://tpolecat.github.io/2014/04/11/scalac-flags.html
  // These needs to separately be configured in Idea
  scalacOptions ++= Seq("-deprecation", "-feature", "-Xfatal-warnings", "-language:implicitConversions"),
  resolvers ++= Seq(
    "Sonatype snapshots" at "https://oss.sonatype.org/content/repositories/snapshots/",
    "scalaz-bintray" at "https://dl.bintray.com/scalaz/releases"
  ),
  libraryDependencies := common ++ commonServerDependencies
)

lazy val root = (project in file("."))
  .settings(
    commonSettings: _*
  )
