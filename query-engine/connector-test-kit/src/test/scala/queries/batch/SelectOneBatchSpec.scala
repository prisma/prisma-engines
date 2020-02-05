package queries.batch

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class SelectOneBatchSpec extends FlatSpec with Matchers with ApiSpecBase {
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
        |                         Name: "ArtistWithoutAlbums"
        |                         ArtistId: 1
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithAlbumButWithoutTracks {createArtist(data:{
        |                         Name: "ArtistWithOneAlbumWithoutTracks"
        |                         ArtistId: 2,
        |                         Albums: {create: [
        |                                   {Title: "TheAlbumWithoutTracks",
        |                                    AlbumId: 2
        |                          }]}
        |}){Name}}""",
      project = project
    )
  }

  "one successful query" should "work" in {
    server.batch(Array("""query {artist(where:{ArtistId: 1}){Name}}"""), project).toString should be(
      """[{"data":{"artist":{"Name":"ArtistWithoutAlbums"}}}]"""
    )
  }

  "two successful queries" should "work" in {
    val queries = Array(
      """query {artist(where:{ArtistId: 1}){Name}}""",
      """query {artist(where:{ArtistId: 2}){Name}}""",
    )

    server.batch(queries, project).toString should be(
      """[{"data":{"artist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"artist":{"Name":"ArtistWithOneAlbumWithoutTracks"}}}]"""
    )
  }

  "one singular failing query" should "work" in {
    server.batch(Array("""query {artist(where:{ArtistId: 420}){Name}}"""), project).toString should be(
      """[{"data":{"artist":null}}]"""
    )
  }

  "one singular failing query out of two" should "work" in {
    val queries = Array(
      """query {artist(where:{ArtistId: 1}){Name}}""",
      """query {artist(where:{ArtistId: 420}){Name}}""",
    )

    server.batch(queries, project).toString should be(
      """[{"data":{"artist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"artist":null}}]"""
    )
  }
}
