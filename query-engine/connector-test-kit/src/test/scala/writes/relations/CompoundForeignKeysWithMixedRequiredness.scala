package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

// RS: Ported
class CompoundForeignKeysWithMixedRequiredness extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "A One to Many relation with mixed requiredness" should "be writable and readable" taggedAs (IgnoreMySql56) in {
    //MySQL 5.6 fills Not Null columns without a default with an implicit default when not in strict mode -.-

    val testDataModels = {
      val dm1 = """
                  model Post {
                      id       Int   @id
                      user_id  Int
                      user_age Int?
                      User     User? @relation(fields: [user_id, user_age], references: [nr, age])

                  }

                  model User {
                      id   Int    @id
                      nr   Int
                      age  Int
                      Post Post[]

                      @@unique([nr, age], name: "user_unique")
                  }"""

      TestDataModels(mongo = dm1, sql = dm1)
    }

    testDataModels.testV11 { project =>
      //Setup user
      server.query("mutation{createUser(data:{id: 1, nr:1, age: 1}){id, nr, age, Post{id}}}", project).toString() should be(
        """{"data":{"createUser":{"id":1,"nr":1,"age":1,"Post":[]}}}""")

      //Null constraint violation
      server.queryThatMustFail("mutation{createPost(data:{id: 1}){id, user_id, user_age, User{id}}}", project, errorCode = 2011)

      //Success
      server.query("mutation{createPost(data:{id: 1, user_id:1}){id, user_id, user_age, User{id}}}", project).toString() should be(
        """{"data":{"createPost":{"id":1,"user_id":1,"user_age":null,"User":null}}}""")

      //Foreign key violation
      server.queryThatMustFail("mutation{createPost(data:{id: 2, user_id:2, user_age: 2}){id, user_id, user_age, User{id}}}", project, errorCode = 2003)

      //Success
      server.query("mutation{createPost(data:{id: 2, user_id:1, user_age: 1}){id, user_id, user_age, User{id}}}", project).toString() should be(
        """{"data":{"createPost":{"id":2,"user_id":1,"user_age":1,"User":{"id":1}}}}""")

    }
  }
}
