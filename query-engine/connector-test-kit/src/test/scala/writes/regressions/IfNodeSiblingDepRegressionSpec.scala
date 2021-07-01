package writes.regressions

import org.scalatest.{FlatSpec, Matchers}
import util._

// RS: Ported
class IfNodeSiblingDepRegressionSpec extends FlatSpec with Matchers with ApiSpecBase {
  // Related issue: https://github.com/prisma/prisma/issues/4230
  "The if node sibling reordering" should "include all siblings that are not another if" in {
    val project = ProjectDsl.fromString {
      """
        |model Container {
        |  id     Int      @id @default(autoincrement())
        |
        |  Record Record[]
        |}
        |
        |model RecordConfig {
        |  id     Int      @id @default(autoincrement())
        |
        |  Record Record[]
        |}
        |
        |model RecordLocation {
        |  id       Int    @id @default(autoincrement())
        |  location String @unique
        |
        |  Record Record[]
        |}
        |
        |model RecordType {
        |  id     Int      @id @default(autoincrement())
        |  type   String   @unique
        |
        |  Record Record[]
        |}
        |
        |model Record {
        |  id           Int            @id @default(autoincrement())
        |  location     RecordLocation @relation(fields: [locationId], references: [id])
        |  locationId   Int
        |  type         RecordType     @relation(fields: [recordTypeId], references: [id])
        |  recordTypeId Int
        |  config       RecordConfig?  @relation(fields: [configId], references: [id])
        |  configId     Int?
        |  container    Container      @relation(fields: [containerId], references: [id])
        |  containerId  Int
        |}
      """.stripMargin
    }
    database.setup(project)

    // Setup
    server.query(
      """
        |mutation {
        |  createOneRecordConfig(data: {}) {id}
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    server.query(
      """
        |mutation {
        |  createOneContainer(data: {}) {id}
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    val result = server.query(
      """
        |mutation {
        |  createOneRecord(data:{
        |    container: { connect: { id: 1 }}
        |    config: { connect: { id: 1 }}
        |  	location: {
        |      connectOrCreate: {
        |        where: { location: "something" }
        |        create: { location: "something" }
        |      }
        |    }
        |    type: {
        |      connectOrCreate: {
        |        where: { type: "test" }
        |        create: { type: "test" }
        |      }
        |    }
        |  }) {
        |    id
        |  }
        |}
      """.stripMargin,
      project,
      legacy = false
    )

    result.toString() should be("""{"data":{"createOneRecord":{"id":1}}}""")
  }
}
