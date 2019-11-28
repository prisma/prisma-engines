package queries

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

class MultiFieldUniqueQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  val schema = """model User {
                |  id        String @id @default(cuid())
                |  FirstName String
                |  LastName  String
                |
                |  @@unique([FirstName, LastName])
                |}
              """.stripMargin

  def createUser(project: Project, firstName: String, lastName: String): String = {
    server
          .query(s"""mutation {
            |  createUser(data: {FirstName: "$firstName", LastName: "$lastName"}) {
            |    id
            |  }
            |}""".stripMargin, project)
          .pathAsString("data.createUser.id")
  }

  "A simple multi-field-unique query" should "work" in {
    val project = SchemaDsl.fromStringV11() { schema }
    database.setup(project)

    val userId = createUser(project, "Hans", "Wurst")
    val result = server.query(s"""{
        |  user(where: {FirstName_LastName: {
        |    FirstName: "Hans"
        |    LastName: "Wurst"
        |  }}){
        |    id
        |  }
        |}""".stripMargin,
                              project)

    result.pathAsString("data.user.id") should equal(userId)
  }

  "A simple multi-field-unique query on a nonexistent user" should "return null" in {
    val project = SchemaDsl.fromStringV11() { schema }
    database.setup(project)

    val result = server.query(s"""{
                                 |  user(where: {FirstName_LastName: {
                                 |    FirstName: "Foo"
                                 |    LastName: "Bar"
                                 |  }}){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project)

    result.pathAsJsValue("data.user") should equal(JsNull)
  }
}
