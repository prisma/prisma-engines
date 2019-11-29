package writes

import org.scalatest.{FlatSpec, Matchers}
import util._

class MultiFieldUniqueQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  "A simple multi-field-unique query" should "work" in {}
}
//"A multi-field-unique query on a one to many relation" should "work" in {
//  val project = SchemaDsl.fromStringV11() { """model User {
//                                              |  id        String @id @default(cuid())
//                                              |  blogs     Blog[]
//                                              |}
//                                              |
//                                              |model Blog {
//                                              |  id       String @id @default(cuid())
//                                              |  title    String
//                                              |  category String
//                                              |  author   User?
//                                              |
//                                              |  @@unique([title, category])
//                                              |}
//                                            """.stripMargin }
//  database.setup(project)
//
//  val userId = server
//    .query(
//      s"""mutation {
//         |  createUser(data: {blogs: {
//         |    create: {
//         |      title: "TestTitle"
//         |      category: "TestContent"
//         |    }
//         |  }}) {
//         |    id
//         |  }
//         |}""".stripMargin,
//      project
//    )
//    .pathAsString("data.createUser.id")
//
//  val result = server.query(
//    s"""{
//       |  user(where: {blogs: {
//       |    FirstName: "Hans"
//       |    LastName: "Wurst"
//       |  }}){
//       |    id
//       |  }
//       |}""".stripMargin,
//    project
//  )
//
//  result.pathAsString("data.user.id") should equal(userId)
//}
