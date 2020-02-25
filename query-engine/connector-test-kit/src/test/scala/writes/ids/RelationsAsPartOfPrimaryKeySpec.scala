package writes.ids

import org.scalatest.{FlatSpec, Matchers}
import util._

class RelationsAsPartOfPrimaryKeySpec extends FlatSpec with Matchers with ApiSpecBase {

  // Does the to one side have to be required??

  //todo relation cardinalities
  // 1!:1!
  // 1!:1
  // 1:!1
  // 1:M
  // 1!:M

  //todo relation variants
  // single id is also a relation
  // compound id contains simple relation + scalar
  // compound id contains compound relation field + scalar
  // compound id contains all compound relation fields + scalar
  // compound id is subset of compound relation field            (unlikely)

  //todo possible mutations
  // create                           | toplevel
  // update                           | toplevel
  // update many                      | toplevel
  // upsert                           | toplevel
  // nested create                    | create
  // nested connect                   | create
  // nested create                    | update
  // nested update                    | update
  // nested upsert                    | update
  // nested connect                   | update
  // nested disconnect                | update
  // nested set                       | update
  // nested delete                    | update
  // nested create                    | upsert
  // nested update                    | upsert
  // nested upsert                    | upsert
  // nested connect                   | upsert
  // nested disconnect                | upsert
  // nested set                       | upsert
  // nested delete                    | upsert
  // delete many                      | toplevel
  // delete                           | toplevel

  "Using a simple id that is also a relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child   Child   @relation(references: [id]) @id
         |  name    String 
         |  age     Int
         |}
         |
         |model Child {
         |  id      Int @id
         |  name    String
         |  parent  Parent
         |}
       """
    }
    database.setup(project)
    server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 10, child: {create: {id: 1, name: "Peter"}}}){
        |    name
        |    age
        |    child{
        |       id
        |       name
        |    }
        |  }
        |}
      """,
      project
    )

  }

  "Using a compound id part of which is a relation" should "work" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Parent {
         |  child   Child   @relation(references: [id])
         |  name    String 
         |  age     Int
         |  
         |  @@id([child, name])
         |}
         |
         |model Child {
         |  id      Int @id
         |  name    String
         |  parent  Parent []
         |}
       """
    }
    database.setup(project)

  }

}
