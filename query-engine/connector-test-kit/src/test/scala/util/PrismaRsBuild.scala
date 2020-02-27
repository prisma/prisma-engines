package util

object PrismaRsBuild {
  val isDebug = false

  def apply(): Unit = {
    if (!EnvVars.isBuildkite) {
      val workingDirectory = new java.io.File(EnvVars.serverRoot)
      val command = if (isDebug) {
        Seq("cargo", "build", "--bin", "prisma", "--bin", "migration-engine")
      } else {
        Seq("cargo", "build", "--release", "--bin", "prisma", "--bin", "migration-engine")
      }

      val env = ("RUST_LOG", "error")
      sys.process.Process(command, workingDirectory, env).!!
    }
  }
}
