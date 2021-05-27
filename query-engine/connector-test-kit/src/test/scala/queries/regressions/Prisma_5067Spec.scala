package queries.regressions

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

//RS: Won't port? Doesn't seem to do anything?
class Prisma_5067Spec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  // validates fix for
  // https://github.com/prisma/prisma/issues/4146

  // Updating a many-to-many relationship using connect should update the
  // `updatedAt` field on the side where the relationship is embedded.

  val project = SchemaDsl.fromStringV11() {
     """
     | model User {
     |   id    String @default(cuid()) @id
     |   posts Post[] @relation("UserToPosts")
     | }
     | 
     | model Post {
     |   id        String     @default(cuid()) @id
     |   author    User       @relation("UserToPosts", fields: [authorId], references: [id])
     |   authorId  String
     |   someField SomeModel?
     | }
     | 
     | model SomeModel {
     |   id     String @id @default(cuid())
     |   post   Post   @relation(fields: [postId], references: [id])
     |   postId String
     | }
     """.stripMargin
  }

  "Updating a list of fields over a connect bound" should "change the update fields tagged with @updatedAt" in {
    database.setup(project)

    server.query(
      s"""query {
      |  findManyUser(where: { posts: { some: { someField: null } } }) {
      |    id
      |  }
      |}
      """.stripMargin,
      project,
      legacy = false,
    )
  }
}
