package util

object PrismaRsBuild {
  val isDebug = false

  def apply(): Unit = {
    if (!EnvVars.isBuildkite) {
      val workingDirectory = new java.io.File(EnvVars.serverRoot)
      var command          = Seq("cargo", "build", "--bin", "prisma", "--bin", "migration-engine")

      if (!isDebug) {
        command = command :+ "--release"
      }

      val env = ("RUST_LOG", "info")
      sys.process.Process(command, workingDirectory, env).!!
    }
  }
}
