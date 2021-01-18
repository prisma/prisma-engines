package util

object PrismaRsBuild {
  val buildMode = sys.env.getOrElse("TEST_BINARY_BUILD_MODE", "release")

  def apply(): Unit = {
    if (!EnvVars.isBuildkite) {
      build("query-engine", "query-engine/query-engine/Cargo.toml")
      build("migration-engine", "migration-engine/cli/Cargo.toml")
    }
  }

  private def build(binary: String, manifestPath: String): Unit = {
    val workingDirectory = new java.io.File(EnvVars.serverRoot)
    var command          = Seq("cargo", "build", "--bin", binary, "--features", "quaint/vendored-openssl", "--manifest-path", manifestPath)

    if (buildMode == "release") {
      command = command :+ "--release"
    }

    val env = ("RUST_LOG", "info")
    sys.process.Process(command, workingDirectory, env).!!
  }
}


