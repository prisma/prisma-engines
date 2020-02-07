package util

object EnvVars {
  val serverRoot = sys.env
    .get("SERVER_ROOT")
    .orElse(sys.env.get("BUILDKITE_BUILD_CHECKOUT_PATH").map(path => s"$path/server")) // todo change as soon as the split is done
    .getOrElse(sys.error("Unable to resolve cargo root path"))

  // compatibility with `test_connector.sh`
  val targetDirectory = sys.env.getOrElse("ABSOLUTE_CARGO_TARGET_DIR", s"$serverRoot/target")
  val binaryDirectory = if (PrismaRsBuild.isDebug) {
    s"$targetDirectory/debug"
  } else {
    s"$targetDirectory/release"
  }

  val prismaBinaryPath                  = s"$binaryDirectory/prisma"
  val migrationEngineBinaryPath: String = s"$binaryDirectory/migration-engine"

  val isBuildkite = sys.env.get("IS_BUILDKITE").isDefined
}
