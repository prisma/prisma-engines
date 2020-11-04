package util

object PrismaRsBuild {
  val buildMode = sys.env.getOrElse("TEST_BINARY_BUILD_MODE", "release")

  def apply(): Unit = {
    if (!EnvVars.isBuildkite) {
      val workingDirectory = new java.io.File(EnvVars.serverRoot)
      var command          = Seq("cargo", "build", "--bin", "query-engine", "--bin", "migration-engine")

      if (buildMode == "release") {
        command = command :+ "--release"
      }

      val env = ("RUST_LOG", "info")
      sys.process.Process(command, workingDirectory, env).!!
    }
  }
}
