package queries.batch

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class InSelectionBatching extends FlatSpec with Matchers with ApiSpecBase {
  val project = ProjectDsl.fromString {
    """model Artist {
      |  id       String @id @default(cuid())
      |  ArtistId Int    @unique
      |  Name     String
      |  Albums   Album[]
      |}
      |
      |model Album {
      |  id      String  @id @default(cuid())
      |  AlbumId Int     @unique
      |  Title   String
      |  Artist  Artist  @relation(references: [id])
      |  @@index([Artist])
      |}
      |"""
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)

    server.query(
      """mutation artistWithoutAlbums {createArtist(data:{
        |  Name: "ArtistWithoutAlbums"
        |  ArtistId: 1
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithAlbumButWithoutTracks {createArtist(data:{
        |  Name: "ArtistWithOneAlbumWithoutTracks"
        |  ArtistId: 2,
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithAlbumButWithoutTracks {createArtist(data:{
        |  Name: "Three"
        |  ArtistId: 3,
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithAlbumButWithoutTracks {createArtist(data:{
        |  Name: "Four"
        |  ArtistId: 4,
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithAlbumButWithoutTracks {createArtist(data:{
        |  Name: "Five"
        |  ArtistId: 5,
        |}){Name}}""",
      project = project
    )
  }

  "batching of IN queries" should "work when having more than the specified amount of items" in {
    val res = server.query(
      """query idInTest {
        |   findManyArtist(where: { ArtistId_in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }) { ArtistId }
        |}
        |""".stripMargin,
      project = project,
      legacy = false,
      batchSize = 2,
    )

    res.toString should be(
      """{"data":{"findManyArtist":[{"ArtistId":1},{"ArtistId":2},{"ArtistId":3},{"ArtistId":4},{"ArtistId":5}]}}""".stripMargin
    )
  }

  "ordering of batched IN queries" should "work when having more than the specified amount of items" in {
    val res = server.query(
      """query idInTest {
        |   findManyArtist(where: { ArtistId_in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }, orderBy: ArtistId_DESC) { ArtistId }
        |}
        |""".stripMargin,
      project = project,
      legacy = false,
      batchSize = 2,
    )

    res.toString should be(
      """{"data":{"findManyArtist":[{"ArtistId":5},{"ArtistId":4},{"ArtistId":3},{"ArtistId":2},{"ArtistId":1}]}}""".stripMargin
    )
  }
}
