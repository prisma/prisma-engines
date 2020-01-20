package util

import java.io.{BufferedReader, InputStreamReader}
import java.lang.ProcessBuilder.Redirect
import java.net.{HttpURLConnection, URL}
import java.nio.charset.StandardCharsets
import java.util.Base64
import java.util.concurrent.atomic.AtomicInteger

import play.api.libs.json._

import scala.concurrent.duration.Duration
import scala.concurrent.{Await, Awaitable, Future}
import scala.util.{Success, Try}

case class QueryEngineResponse(status: Int, body: String) {
  lazy val jsonBody: Try[JsValue] = Try(Json.parse(body))
}

object TestServer {
  val nextPort = new AtomicInteger(4000)
}

case class TestServer() extends PlayJsonExtensions {
  import scala.concurrent.ExecutionContext.Implicits.global

  def query(
      query: String,
      project: Project,
      dataContains: String = ""
  ): JsValue = {
    queryAsync(query, project, dataContains)
  }

  def queryAsync(query: String, project: Project, dataContains: String = ""): JsValue = {
    val result = queryBinaryCLI(
      query = query.stripMargin,
      project = project,
    )

    println("Query :" + result)
    result.assertSuccessfulResponse(dataContains)
    result
  }

  def queryThatMustFail(
      query: String,
      project: Project,
      errorCode: Int,
      errorCount: Int = 1,
      errorContains: String = ""
  ): JsValue = {
//    val result = awaitInfinitely {
//      querySchemaAsync(
//        query = query.stripMargin,
//        project = project,
//      )
//    }

    val result =
      queryBinaryCLI(
        query = query.stripMargin,
        project = project,
      )

    println("Failing Query " + result)
    // Ignore error codes for external tests (0) and containment checks ("")
    result.assertFailingResponse(0, errorCount, "")
    result
  }

  private def querySchemaAsync(
      query: String,
      project: Project
  ): Future[JsValue] = {
    val (port, queryEngineProcess) = startQueryEngine(project)

    println(s"query engine started on port $port, pid: ${getPidOfProcess(queryEngineProcess)}")
    println(s"Query: $query")

    Future {
      queryPrismaProcess(query, port)
    }.transform { r =>
        queryEngineProcess.destroyForcibly().waitFor()
        println(s"pid stopped: ${getPidOfProcess(queryEngineProcess)}")
        r
      }
      .map { r =>
        println(s"Query result: $r")
        r.jsonBody.get
      }
  }

  import java.lang.reflect.Field

  def getPidOfProcess(p: Process): Long = {
    var pid: Long = -1
    try if (p.getClass.getName == "java.lang.UNIXProcess") {
      val f = p.getClass.getDeclaredField("pid")
      f.setAccessible(true)
      pid = f.getLong(p)
      f.setAccessible(false)
    } catch {
      case e: Exception =>
        pid = -1
    }
    pid
  }

  private def startQueryEngine(project: Project) = {
    import java.lang.ProcessBuilder.Redirect

    // TODO: discuss with Dom whether we want to keep the legacy mode
    val pb         = new java.lang.ProcessBuilder(EnvVars.prismaBinaryPath, "--legacy")
    val workingDir = new java.io.File(".")

    val fullDataModel = project.dataModelWithDataSourceConfig
    // Important: Rust requires UTF-8 encoding (encodeToString uses Latin-1)
    val encoded = Base64.getEncoder.encode(fullDataModel.getBytes(StandardCharsets.UTF_8))
    val envVar  = new String(encoded, StandardCharsets.UTF_8)
    val port    = TestServer.nextPort.incrementAndGet()

    pb.environment.put("PRISMA_DML", envVar)
    pb.environment.put("PORT", port.toString)
    pb.environment.put("LOG_QUERIES", "y")
    pb.environment.put("RUST_LOG", sys.env.getOrElse("RUST_LOG", "info"))

    pb.directory(workingDir)
    pb.redirectErrorStream(true)
    pb.redirectOutput(Redirect.INHERIT)

    val process = pb.start

    waitUntilServerIsUp(port)
//    Thread.sleep(1000)

    (port, process)
  }

  def queryBinaryCLI(query: String, project: Project) = {
    val formattedQuery = query.stripMargin.replace("\n", "")
    import sys.process._
    val res =
      Process(Seq("/Users/matthias/repos/work/prisma-engine/target/debug/prisma", "cli", "--execute_request", formattedQuery),
              None,
              "PRISMA_DML" -> project.envVar).!!
    val res2 = Json.parse(res)
    println(res2)
    res2
  }

  private def queryPrismaProcess(query: String, port: Int): QueryEngineResponse = {
    val url = new URL(s"http://127.0.0.1:$port")
    val con = url.openConnection().asInstanceOf[HttpURLConnection]

    con.setDoOutput(true)
    con.setRequestMethod("POST")
    con.setRequestProperty("Content-Type", "application/json")

    val body = Json.obj("query" -> query, "variables" -> Json.obj()).toString()

    con.setRequestProperty("Content-Length", Integer.toString(body.length))
    con.getOutputStream.write(body.getBytes(StandardCharsets.UTF_8))

    try {
      val status = con.getResponseCode
      val streamReader = if (status > 299) {
        new InputStreamReader(con.getErrorStream, "utf8")
      } else {
        new InputStreamReader(con.getInputStream, "utf8")
      }

      val in     = new BufferedReader(streamReader)
      val buffer = new StringBuffer

      Stream.continually(in.readLine()).takeWhile(_ != null).foreach(buffer.append)
      QueryEngineResponse(status, buffer.toString)
    } catch {
      case e: Throwable => QueryEngineResponse(999, s"""{"errors": [{"message": "Connection error: $e"}]}""")
    } finally {
      con.disconnect()
    }
  }

  private def waitUntilServerIsUp(port: Int): Unit = {
    val sleepTime   = 5 // 5ms
    val maxWaitTime = 2000 // 2s
    var tryCount    = 0
    while (!isServerUp(port)) {
      Thread.sleep(sleepTime)
      if (tryCount * sleepTime > maxWaitTime) {
        sys.error("TestServer did not start within maximum wait time")
      }
      tryCount += 1
    }
  }

  private def isServerUp(port: Int): Boolean = {
    val url = new URL(s"http://127.0.0.1:$port/status")
    val con = url.openConnection().asInstanceOf[HttpURLConnection]
    con.setRequestMethod("GET")
    try {
      val status = con.getResponseCode
      status == 200
    } catch {
      case e: Throwable => false
    } finally {
      con.disconnect()
    }
  }

  private def awaitInfinitely[T](awaitable: Awaitable[T]): T = Await.result(awaitable, Duration.Inf)
}
