package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util._

class UpdateManySpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = ProjectDsl.fromString {
    """model TestModel {
      |  id       String  @id @default(cuid())
      |  optStr   String?
      |  optInt   Int?
      |  optFloat Float?
      |}
    """.stripMargin
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  override def beforeEach(): Unit = database.truncateProjectTables(project)

  "An updateMany mutation" should "update the records matching the where clause" in {
    createTestModel("str1")
    createTestModel("str2")

    var result = server.query(
      """mutation {
        |  updateManyTestModel(
        |    where: { optStr: { equals: "str1" } }
        |    data: { optStr: { set: "str1new" }, optInt: { set: 1 }, optFloat: { multiply: 2 } }
        |  ) {
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsLong("data.updateManyTestModel.count") should equal(1)

    result = server.query(
      """{
        |  findManyTestModel(orderBy: { id: asc }) {
        |    optStr
        |    optInt
        |    optFloat
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.findManyTestModel").toString should be(
      """[{"optStr":"str1new","optInt":1,"optFloat":null},{"optStr":"str2","optInt":null,"optFloat":null}]""")
  }

  "An updateMany mutation" should "update all items if the where clause is empty" in {
    createTestModel("str1")
    createTestModel("str2", Some(2))
    createTestModel("str3", Some(3), Some(3.1))

    var result = server.query(
      """mutation {
        |  updateManyTestModel(
        |    where: { }
        |    data: { optStr: { set: "updated" }, optFloat: { divide: 2 }, optInt: { decrement: 1 } }
        |  ){
        |    count
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsLong("data.updateManyTestModel.count") should equal(3)

    result = server.query(
      """{
        |  findManyTestModel {
        |    optStr
        |    optInt
        |    optFloat
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false,
    )

    result.pathAsJsValue("data.findManyTestModel").toString should be(
      """[{"optStr":"updated","optInt":null,"optFloat":null},{"optStr":"updated","optInt":1,"optFloat":null},{"optStr":"updated","optInt":2,"optFloat":1.55}]""")
  }

//  "UpdateMany" should "work between top level types" in {
//
//    val project = ProjectDsl.fromString {
//      """
//        |model ZChild{
//        |    id       String  @id @default(cuid())
//        |    name     String? @unique
//        |    test     String?
//        |    parentId String?
//        |
//        |    parent  Parent? @relation(fields: [parentId], references: [id])
//        |}
//        |
//        |model Parent{
//        |    id       String   @id @default(cuid())
//        |    name     String?  @unique
//        |    children ZChild[]
//        |}"""
//    }
//
//    database.setup(project)
//
//    val create = server.query(
//      s"""mutation {
//         |   createParent(data: {
//         |   name: "Dad",
//         |   children: {create:[{ name: "Daughter"},{ name: "Daughter2"}, { name: "Son"},{ name: "Son2"}]}
//         |}){
//         |  name,
//         |  children{ name}
//         |}}""",
//      project
//    )
//
//    create.toString should be(
//      """{"data":{"createParent":{"name":"Dad","children":[{"name":"Daughter"},{"name":"Daughter2"},{"name":"Son"},{"name":"Son2"}]}}}""")
//
//    val nestedUpdateMany = server.query(
//      s"""mutation {
//         |   updateParent(
//         |   where: { name: "Dad" }
//         |   data: {  children: {updateMany:[
//         |      {
//         |          where:{name: { contains:"Daughter" }}
//         |          data:{test: { set: "UpdateManyDaughters"} }
//         |      },
//         |      {
//         |          where:{name: { contains:"Son" }}
//         |          data:{test: { set: "UpdateManySons" }}
//         |      }
//         |   ]
//         |  }}
//         |){
//         |  name,
//         |  children{ name, test}
//         |}}""",
//      project
//    )
//
//    nestedUpdateMany.toString should be(
//      """{"data":{"updateParent":{"name":"Dad","children":[{"name":"Daughter","test":"UpdateManyDaughters"},{"name":"Daughter2","test":"UpdateManyDaughters"},{"name":"Son","test":"UpdateManySons"},{"name":"Son2","test":"UpdateManySons"}]}}}""")
//  }

  def createTestModel(optStr: String, optInt: Option[Int] = None, optFloat: Option[Double] = None): Unit = {
    val f = optFloat match {
      case Some(o) => s"$o"
      case None    => "null"
    }

    val i = optInt match {
      case Some(o) => s"$o"
      case None    => "null"
    }

    server.query(
      s"""
         |mutation {
         |  createOneTestModel(
         |    data: {
         |      optStr: "$optStr"
         |      optInt: $i
         |      optFloat: $f
         |    }
         |  ) {
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      legacy = false,
    )
  }
}
