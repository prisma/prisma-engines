package queries.batch

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class SelectOneSingularIdBatchSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = ProjectDsl.fromString {
    """model Artist {
      |  id       String  @id @default(cuid())
      |  ArtistId Int     @unique
      |  Name     String
      |  Albums   Album[]
      |}
      |
      |model Album {
      |  id       String  @id @default(cuid())
      |  AlbumId  Int     @unique
      |  Title    String
      |  ArtistId String
      |
      |  Artist  Artist  @relation(fields: [ArtistId], references: [id])
      |  @@index([ArtistId])
      |}
      """
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
    server.batch(Array("""query {findOneArtist(where:{ArtistId: 1}){Name}}"""), project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Name":"ArtistWithoutAlbums"}}}]"""
    )
  }

  "two successful queries and one failing with same selection set" should "work" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId: 1}){Name, ArtistId}}""",
      """query {findOneArtist(where:{ArtistId: 420}){Name, ArtistId}}""",
      """query {findOneArtist(where:{ArtistId: 2}){ArtistId, Name}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Name":"ArtistWithoutAlbums","ArtistId":1}}},{"data":{"findOneArtist":null}},{"data":{"findOneArtist":{"Name":"ArtistWithOneAlbumWithoutTracks","ArtistId":2}}}]"""
    )
  }

  "two successful queries and one failing with different selection set" should "work" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId: 1}){ArtistId, Name}}""",
      """query {findOneArtist(where:{ArtistId: 420}){Name}}""",
      """query {findOneArtist(where:{ArtistId: 2}){Name, ArtistId}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"ArtistId":1,"Name":"ArtistWithoutAlbums"}}},{"data":{"findOneArtist":null}},{"data":{"findOneArtist":{"Name":"ArtistWithOneAlbumWithoutTracks","ArtistId":2}}}]"""
    )
  }

  "joins and such" should "just work" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId:2}) {Albums { AlbumId, Title }}}""",
      """query {findOneArtist(where:{ArtistId:1}) {Albums { Title, AlbumId }}}""",
      """query {findOneArtist(where:{ArtistId:420}) {Albums { AlbumId, Title }}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findOneArtist":{"Albums":[]}}},{"data":{"findOneArtist":null}}]"""
    )
  }

  "joins with same conditions" should "just work" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId:2}) {Albums(where:{AlbumId:2}) { AlbumId, Title }}}""",
      """query {findOneArtist(where:{ArtistId:1}) {Albums(where:{AlbumId:2}) { Title, AlbumId }}}""",
      """query {findOneArtist(where:{ArtistId:420}) {Albums(where:{AlbumId:2}) { AlbumId, Title }}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findOneArtist":{"Albums":[]}}},{"data":{"findOneArtist":null}}]"""
    )
  }

  "joins with different conditions" should "just work" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId:2}) {Albums(where:{AlbumId:2}) { AlbumId, Title }}}""",
      """query {findOneArtist(where:{ArtistId:1}) {Albums(where:{AlbumId:1}) { Title, AlbumId }}}""",
      """query {findOneArtist(where:{ArtistId:420}) {Albums(where:{AlbumId:2}) { AlbumId, Title }}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Albums":[{"AlbumId":2,"Title":"TheAlbumWithoutTracks"}]}}},{"data":{"findOneArtist":{"Albums":[]}}},{"data":{"findOneArtist":null}}]"""
    )
  }

  "one singular failing query" should "work" in {
    server.batch(Array("""query {findOneArtist(where:{ArtistId: 420}){Name}}"""), project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":null}}]"""
    )
  }

  "one singular failing query out of two" should "work" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId: 1}){Name}}""",
      """query {findOneArtist(where:{ArtistId: 420}){Name}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"findOneArtist":null}}]"""
    )
  }

  "two queries that are the same" should "return answers for both of the queries" in {
    val queries = Array(
      """query {findOneArtist(where:{ArtistId: 1}){Name}}""",
      """query {findOneArtist(where:{ArtistId: 1}){Name}}""",
    )

    server.batch(queries, project, legacy = false).toString should be(
      """[{"data":{"findOneArtist":{"Name":"ArtistWithoutAlbums"}}},{"data":{"findOneArtist":{"Name":"ArtistWithoutAlbums"}}}]"""
    )
  }
}
