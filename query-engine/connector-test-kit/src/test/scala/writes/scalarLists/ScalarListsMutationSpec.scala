package writes.scalarLists

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.{JoinRelationLinksCapability, ScalarListsCapability}
import util._

class ScalarListsMutationSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(ScalarListsCapability, JoinRelationLinksCapability)

  "Nested scalar lists" should "work in creates " in {
    val testDataModels = {
      val s1 = s"""model List{
                  |   id String @id @default(cuid())
                  |   todos Todo[] @relation(references: [id])
                  |   listInts Int[]
                  |}
                  |
                  |model Todo{
                  |   id String @id @default(cuid())
                  |   lists List[]
                  |   todoInts Int[]
                  |}"""

      val s2 = s"""model List{
                  |   id String @id @default(cuid())
                  |   todos Todo[]
                  |   listInts Int[]
                  |}
                  |
                  |model Todo{
                  |   id String @id @default(cuid())
                  |   lists List[]
                  |   todoInts Int[]
                  |}"""
      TestDataModels(mongo = Vector(s1), sql = Vector(s2))
    }

    testDataModels.testV11 { project =>
      server.query(
        s"""mutation{createList(data: {listInts: {set: [1, 2]}, todos: {create: {todoInts: {set: [3, 4]}}}}) {id}}""".stripMargin,
        project
      )

      val result = server.query(s"""query{lists {listInts, todos {todoInts}}}""".stripMargin, project)

      result.toString should equal("""{"data":{"lists":[{"listInts":[1,2],"todos":[{"todoInts":[3,4]}]}]}}""")
    }

  }

  "Deeply nested scalar lists" should "work in creates " in {
    val testDataModels = {
      val s1 = s"""model List{
                  |   id       String @id @default(cuid())
                  |   listInts Int[]
                  |   todoId   String?
                  |
                  |   todo Todo? @relation(fields: [todoId], references: [id])
                  |}
                  |
                  |model Todo{
                  |   id       String @id @default(cuid())
                  |   todoInts Int[]
                  |   tagId    String?
                  |
                  |   list List?
                  |   tag  Tag?  @relation(fields: [tagId], references: [id])
                  |}
                  |
                  |model Tag{
                  |   id String @id @default(cuid())
                  |   todo Todo?
                  |   tagInts Int[]
                  |}
                  |"""

      TestDataModels(mongo = Vector(s1), sql = Vector(s1))
    }

    testDataModels.testV11 { project =>
      server.query(
        s"""mutation{createList(data: {listInts: {set: [1, 2]}, todo: {create: {todoInts: {set: [3, 4]}, tag: {create: {tagInts: {set: [5, 6]}}}}}}) {id}}""".stripMargin,
        project
      )

      val result = server.query(s"""query{lists {listInts, todo {todoInts, tag {tagInts}}}}""".stripMargin, project)

      result.toString should equal("""{"data":{"lists":[{"listInts":[1,2],"todo":{"todoInts":[3,4],"tag":{"tagInts":[5,6]}}}]}}""")
    }

  }

  "Deeply nested scalar lists" should "work in updates " in {
    val testDataModels = {
      val s1 = s"""model List{
                  |   id       String @id @default(cuid())
                  |   listInts Int[]
                  |   uList    String @unique
                  |   todoId   String?
                  |
                  |   todo Todo? @relation(fields: [todoId], references: [id])
                  |}
                  |
                  |model Todo{
                  |   id       String @id @default(cuid())
                  |   uTodo    String @unique
                  |   list     List?
                  |   todoInts Int[]
                  |   tagId    String?
                  |
                  |   tag Tag? @relation(fields: [tagId], references: [id])
                  |}
                  |
                  |model Tag{
                  |   id      String @id @default(cuid())
                  |   uTag    String @unique
                  |   tagInts Int[]
                  |
                  |   todo Todo?
                  |}
                  |"""
      TestDataModels(mongo = Vector(s1), sql = Vector(s1))
    }

    testDataModels.testV11 { project =>
      server
        .query(
          s"""mutation{createList(data: {uList: "A", listInts: {set: [1, 2]}, todo: {create: {uTodo: "B", todoInts: {set: [3, 4]}, tag: {create: {uTag: "C",tagInts: {set: [5, 6]}}}}}}) {id}}""".stripMargin,
          project
        )

      server.query(
        s"""mutation{updateList(where: {uList: "A"}
         |                    data: {listInts: {set: [7, 8]},
         |                           todo: {update: {todoInts: {set: [9, 10]},
         |                                           tag: {update: { tagInts: {set: [11, 12]}}}}}}) {id}}""".stripMargin,
        project
      )
      val result = server.query(s"""query{lists {listInts, todo {todoInts, tag {tagInts}}}}""".stripMargin, project)

      result.toString should equal("""{"data":{"lists":[{"listInts":[7,8],"todo":{"todoInts":[9,10],"tag":{"tagInts":[11,12]}}}]}}""")
    }

  }

  "Nested scalar lists" should "work in upserts and only execute one branch of the upsert" in {
    val testDataModels = {
      val s1 = s"""model List{
                  |   id       String @id @default(cuid())
                  |   uList    String @unique
                  |   listInts Int[]
                  |   todoId   String?
                  |
                  |   todo Todo? @relation(fields: [todoId], references: [id])
                  |}
                  |
                  |model Todo{
                  |   id       String @id @default(cuid())
                  |   uTodo    String @unique
                  |   todoInts Int[]
                  |
                  |   list List?
                  |}
                  |"""
      TestDataModels(mongo = Vector(s1), sql = Vector(s1))
    }

    testDataModels.testV11 { project =>
      server
        .query(
          s"""mutation{createList(data: {uList: "A", listInts: {set: [1, 2]}, todo: {create: {uTodo: "B", todoInts: {set: [3, 4]}}}}) {id}}""".stripMargin,
          project
        )
        .pathAsString("data.createList.id")

      server
        .query(
          s"""mutation upsertListValues {upsertList(
          |                             where:{uList: "A"}
          |                             create:{uList:"Should Not Matter" listInts:{set: [75, 85]}}
          |                             update:{listInts:{set: [70, 80]} }
          |){id}}""".stripMargin,
          project
        )
        .pathAsString("data.upsertList.id")

      val result = server.query(s"""query{lists {uList, listInts}}""".stripMargin, project)

      result.toString should equal("""{"data":{"lists":[{"uList":"A","listInts":[70,80]}]}}""")
    }

  }
}
