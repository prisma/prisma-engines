package util

object EnvVars {
  val serverRoot = sys.env
    .get("SERVER_ROOT")
    .orElse(sys.env.get("BUILDKITE_BUILD_CHECKOUT_PATH").map(path => s"$path/server")) // todo change as soon as the split is done
    .getOrElse(sys.error("Unable to resolve cargo root path"))
    .stripSuffix("/")

  // env var is for compatibility with `test_connector.sh`
  val isDebugBuild = sys.env.get("IS_DEBUG_BUILD") match {
    case Some(x) => x == "1"
    case _       => PrismaRsBuild.isDebug
  }

  // env var is for compatibility with `test_connector.sh`
  val targetDirectory           = sys.env.getOrElse("ABSOLUTE_CARGO_TARGET_DIR", s"$serverRoot/target")
  val binaryDirectory           = if (isDebugBuild) s"$targetDirectory/debug" else s"$targetDirectory/release"
  val prismaBinaryPath          = s"$binaryDirectory/query-engine"
  val migrationEngineBinaryPath = s"$binaryDirectory/migration-engine"
  val isBuildkite               = sys.env.get("IS_BUILDKITE").isDefined
  val testMode                  = sys.env.getOrElse("TEST_MODE", "simple")
}
