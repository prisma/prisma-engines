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
  // create                           | toplevel    done
  // update                           | toplevel    done
  // nested create                    | create      done
  // nested update                    | update
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

    val res1 = server.query(
      """
        |mutation {
        |  createParent(data: { name: "Paul" , age: 10, child: {create: {id: 1, name: "Panther"}}}){
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

    res1.toString() should be("")

    val res2 = server.query(
      """
        |mutation {
        |  updateParent(where: {child_name:{child: 1, name: "Paul"}} data: {age: 11}){
        |    name
        |    age
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("")

    val res3 = server.query(
      """
        |mutation {
        |  updateChild(where: {id: 1} data: {parent: {update:{ age 12}}}}){
        |    parent {age}
        |  }
        |}
      """,
      project
    )

    res3.toString() should be("")

    val res4 = server.query(
      """
        |mutation {
        |  deleteParent(where: {child_name:{child: 1, name: "Paul"}}){
        |    name
        |    age
        |  }
        |}
      """,
      project
    )

    res4.toString() should be("")

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
         |  parents  Parent []
         |}
       """
    }
    database.setup(project)

    //todo possible mutations
    // create                           | toplevel
    // update                           | toplevel
    // nested create                    | create
    // nested connect                   | create
    // nested create                    | update
    // nested update                    | update
    // nested connect                   | update
    // nested disconnect                | update
    // nested set                       | update
    // nested delete                    | update
    // delete                           | toplevel

    val res1 = server.query(
      """
        |mutation {
        |  createChild(
        |     data: {
        |         id : 1,
        |         name: "Paul",
        |         parents: {
        |             create:[
        |               {name: Panther, age: 10},
        |               {name: Pawlowski, age: 100},
        |               {name: Parker, age: 1000}
        |             ]
        |        }
        |    }
        | }
        |  ){
        |    parents {age}
        |  }
        |}
      """,
      project
    )

    res1.toString() should be("")

    val res2 = server.query(
      """
        |mutation {
        |  updateChild(
        |     where: {id: 1}
        |     data: {
        |         parents: {
        |             update:{
        |                 where: {child_name: {child: 1, name: "Paul"}
        |                 data: { age 12}
        |                 }
        |              }
        |        }
        |    }
        | }
        |  ){
        |    parents {age}
        |  }
        |}
      """,
      project
    )

    res2.toString() should be("")

  }

}
