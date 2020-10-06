package writes.relations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NonEmbeddedOptionalBackrelationSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "Nested Updates" should "work for models with missing backrelations " in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Owner {
        |  id        String @id @default(cuid())
        |  ownerName String @unique
        |  catId     String
        |
        |  cat       Cat    @relation(fields: [catId], references: [id])
        |}
        |
        |model Cat {
        |  id      String @id @default(cuid())
        |  catName String @unique
        |}
        |
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """mutation {createOwner(data: {ownerName: "jon", cat: {create: {catName: "garfield"}}}) {
        |    ownerName
        |    cat {
        |      catName
        |    }
        |  }
        |}""".stripMargin,
      project
    )

    val res = server.query(
      """mutation {updateOwner(where: {ownerName: "jon"},
        |data: {cat: {update:{catName: {set: "azrael" }}}}) {
        |    ownerName
        |    cat {
        |      catName
        |    }
        |  }
        |}""".stripMargin,
      project
    )

    res.toString() should be("""{"data":{"updateOwner":{"ownerName":"jon","cat":{"catName":"azrael"}}}}""")
  }

  "Nested Upsert" should "work for models with missing backrelations for update " in {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Owner {
        |  id        String @id @default(cuid())
        |  ownerName String @unique
        |  cats      Cat[]  $relationInlineAttribute
        |}
        |
        |model Cat {
        |  id      String @id @default(cuid())
        |  catName String @unique
        |  ownerId String
        |
        |  owner Owner @relation(fields:[ownerId], references:[id])
        |}
        |
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """mutation {createOwner(data: {ownerName: "jon", cats: {create: {catName: "garfield"}}}) {
        |    ownerName
        |    cats {
        |      catName
        |    }
        |  }
        |}""".stripMargin,
      project
    )

    val res = server.query(
      """mutation {updateOwner(where: {ownerName: "jon"},
        |data: {cats: {upsert: {
        |                   where:{catName: "garfield"},
        |                   update: {catName: { set: "azrael"}}
        |                   create: {catName: "should not matter"}
        |                   }}})
        |{
        |    ownerName
        |    cats {
        |      catName
        |    }
        |  }
        |}""".stripMargin,
      project
    )

    res.toString should be("""{"data":{"updateOwner":{"ownerName":"jon","cats":[{"catName":"azrael"}]}}}""")
  }

  "Nested Upsert" should "work for models with missing backrelations for create" in {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Owner {
        |  id        String @id @default(cuid())
        |  ownerName String @unique
        |  cats      Cat[]  $relationInlineAttribute
        |}
        |
        |model Cat {
        |  id      String @id @default(cuid())
        |  catName String @unique
        |  ownerId String
        |
        |  owner Owner @relation(fields:[ownerId], references:[id])
        |}
        |
      """.stripMargin
    }
    database.setup(project)

    server.query(
      """mutation {createOwner(data: {ownerName: "jon", cats: {create: {catName: "garfield"}}}) {
        |    ownerName
        |    cats {
        |      catName
        |    }
        |  }
        |}""".stripMargin,
      project
    )

    val res = server.query(
      """mutation {updateOwner(where: {ownerName: "jon"},
        |data: {cats: {upsert: {
        |                   where:{catName: "DOES NOT EXIST"},
        |                   update: {catName: { set: "SHOULD NOT MATTER" }}
        |                   create: {catName: "azrael"}
        |                   }}})
        |{
        |    ownerName
        |    cats {
        |      catName
        |    }
        |  }
        |}""".stripMargin,
      project
    )

    res.toString should be("""{"data":{"updateOwner":{"ownerName":"jon","cats":[{"catName":"garfield"},{"catName":"azrael"}]}}}""")
  }
}
