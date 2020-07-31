package writes.ids

import org.scalatest.{FlatSpec, Matchers}
import util._

class AutoIncrementCreateSpec extends FlatSpec with Matchers with ApiSpecBase {

  "Creating an item with an id field of type Int without default" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Mail {
         |    id Int   @default(autoincrement())  
         |    messageId Int @id
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createMail(data: { messageId:1 }){
        |    id
        |    messageId
        |  }
        |}
      """.stripMargin,
      project
    )

  }
}
