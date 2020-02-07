package util

object PrismaRsBuild {
  val isDebug = true

  def apply(): Unit = {
    if (!EnvVars.isBuildkite) {
      val workingDirectory = new java.io.File(EnvVars.serverRoot)
      val command = if (isDebug) {
        Seq("cargo", "build")
      } else {
        Seq("cargo", "build", "--release")
      }

      val env = ("RUST_LOG", "error")
      sys.process.Process(command, workingDirectory, env).!!
    }
  }
}
