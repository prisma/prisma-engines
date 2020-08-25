package queries.filters

import org.scalatest._
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class ExtendedRelationFilterSpec extends FlatSpec with Matchers with ApiSpecBase {

  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val project = ProjectDsl.fromString { """model Artist {
                                         |  id       String @id @default(cuid())
                                         |  ArtistId Int    @unique
                                         |  Name     String
                                         |  Albums   Album[]
                                         |}
                                         |
                                         |model Album {
                                         |  id       String  @id @default(cuid())
                                         |  AlbumId  Int     @unique
                                         |  Title    String
                                         |  Tracks   Track[]
                                         |  ArtistId String
                                         |  
                                         |  Artist Artist @relation(fields: [ArtistId], references: [id])
                                         |
                                         |  @@index([ArtistId])
                                         |}
                                         |
                                         |model Genre {
                                         |  id      String @id @default(cuid())
                                         |  GenreId Int    @unique
                                         |  Name    String
                                         |  Tracks  Track[]
                                         |}
                                         |
                                         |model MediaType {
                                         |  id          String @id @default(cuid())
                                         |  MediaTypeId Int    @unique
                                         |  Name        String
                                         |  Tracks      Track[]
                                         |}
                                         |
                                         |model Track {
                                         |  id           String    @id @default(cuid())
                                         |  TrackId      Int       @unique
                                         |  Name         String
                                         |  Composer     String
                                         |  Milliseconds Int
                                         |  Bytes        Int
                                         |  UnitPrice    Float
                                         |  AlbumId      String
                                         |  MediaTypeId  String
                                         |  GenreId      String
                                         |  
                                         |  Album        Album     @relation(fields: [AlbumId], references: [id])
                                         |  MediaType    MediaType @relation(fields: [MediaTypeId], references: [id])
                                         |  Genre        Genre     @relation(fields: [GenreId], references: [id])
                                         |  
                                         |  @@index([AlbumId])
                                         |  @@index([MediaTypeId])
                                         |  @@index([GenreId])
                                         |}
                                         |""" }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)

    // add data
    server.query("""mutation {createGenre(data: {Name: "Genre1", GenreId: 1}){Name}}""", project = project)
    server.query("""mutation {createGenre(data: {Name: "Genre2", GenreId: 2}){Name}}""", project = project)
    server.query("""mutation {createGenre(data: {Name: "Genre3", GenreId: 3}){Name}}""", project = project)
    server.query("""mutation {createGenre(data: {Name: "GenreThatIsNotUsed", GenreId: 4}){Name}}""", project = project)

    server.query("""mutation {createMediaType(data: {Name: "MediaType1", MediaTypeId: 1}){Name}}""", project = project)
    server.query("""mutation {createMediaType(data: {Name: "MediaType2", MediaTypeId: 2}){Name}}""", project = project)
    server.query("""mutation {createMediaType(data: {Name: "MediaType3", MediaTypeId: 3}){Name}}""", project = project)
    server.query("""mutation {createMediaType(data: {Name: "MediaTypeThatIsNotUsed", MediaTypeId: 4}){Name}}""", project = project)

    server.query(
      """mutation completeArtist {createArtist(data:{
        |                         Name: "CompleteArtist"
        |                         ArtistId: 1,
        |                         Albums: {create: [
        |                                   {Title: "Album1",
        |                                    AlbumId: 1,
        |                                    Tracks:{create: [
        |                                             {
        |                                               Name:"Track1",
        |                                               TrackId: 1,
        |                                               Composer: "Composer1",
        |                                               Milliseconds: 10000,
        |                                               Bytes: 512,
        |                                               UnitPrice: 1.51,
        |                                               Genre: {connect: {GenreId: 1}},
        |                                               MediaType: {connect: {MediaTypeId: 1}}
        |                                             }
        |                                    ]}
        |                          }]}
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithoutAlbums {createArtist(data:{
        |                         Name: "ArtistWithoutAlbums"
        |                         ArtistId: 2
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation artistWithAlbumButWithoutTracks {createArtist(data:{
        |                         Name: "ArtistWithOneAlbumWithoutTracks"
        |                         ArtistId: 3,
        |                         Albums: {create: [
        |                                   {Title: "TheAlbumWithoutTracks",
        |                                    AlbumId: 2
        |                          }]}
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation completeArtist2 {createArtist(data:{
        |                         Name: "CompleteArtist2"
        |                         ArtistId: 4,
        |                         Albums: {create: [
        |                                   {Title: "Album3",
        |                                    AlbumId: 3,
        |                                    Tracks:{create: [
        |                                             {
        |                                               Name:"Track2",
        |                                               TrackId: 2,
        |                                               Composer: "Composer1",
        |                                               Milliseconds: 11000,
        |                                               Bytes: 1024,
        |                                               UnitPrice: 2.51,
        |                                               Genre: {connect: {GenreId: 2}},
        |                                               MediaType: {connect: {MediaTypeId: 2}}
        |                                             },
        |                                             {
        |                                               Name:"Track3",
        |                                               TrackId: 3,
        |                                               Composer: "Composer2",
        |                                               Milliseconds: 9000,
        |                                               Bytes: 24,
        |                                               UnitPrice: 5.51,
        |                                               Genre: {connect: {GenreId: 3}},
        |                                               MediaType: {connect: {MediaTypeId: 3}}
        |                                             }
        |                                    ]}
        |                          }]}
        |}){Name}}""",
      project = project
    )

    server.query(
      """mutation completeArtist3 {createArtist(data:{
        |                         Name: "CompleteArtistWith2Albums"
        |                         ArtistId: 5,
        |                         Albums: {create: [
        |                                   {Title: "Album4",
        |                                    AlbumId: 4,
        |                                    Tracks:{create: [
        |                                             {
        |                                               Name:"Track4",
        |                                               TrackId: 4,
        |                                               Composer: "Composer1",
        |                                               Milliseconds: 15000,
        |                                               Bytes: 10024,
        |                                               UnitPrice: 12.51,
        |                                               Genre: {connect: {GenreId: 1}},
        |                                               MediaType: {connect: {MediaTypeId: 1}}
        |                                             },
        |                                             {
        |                                               Name:"Track5",
        |                                               TrackId: 5,
        |                                               Composer: "Composer2",
        |                                               Milliseconds: 19000,
        |                                               Bytes: 240,
        |                                               UnitPrice: 0.51,
        |                                               Genre: {connect: {GenreId: 1}},
        |                                               MediaType: {connect: {MediaTypeId: 1}}
        |                                             }
        |                                           ]}
        |                                   },
        |                                   {Title: "Album5",
        |                                    AlbumId: 5,
        |                                    Tracks:{create: [
        |                                             {
        |                                               Name:"Track6",
        |                                               TrackId: 6,
        |                                               Composer: "Composer1",
        |                                               Milliseconds: 100,
        |                                               Bytes: 724,
        |                                               UnitPrice: 31.51,
        |                                               Genre: {connect: {GenreId: 2}},
        |                                               MediaType: {connect: {MediaTypeId: 3}}
        |                                             },
        |                                             {
        |                                               Name:"Track7",
        |                                               TrackId: 7,
        |                                               Composer: "Composer3",
        |                                               Milliseconds: 100,
        |                                               Bytes: 2400,
        |                                               UnitPrice: 5.51,
        |                                               Genre: {connect: {GenreId: 1}},
        |                                               MediaType: {connect: {MediaTypeId: 1}}
        |                                             }
        |                                    ]}
        |                          }
        |                          ]}
        |}){Name}}""",
      project = project
    )

  }

  "simple scalar filter" should "work" in {
    server.query("""query {artists(where:{ArtistId: { equals: 1 }}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"}]}}""")
  }

  "1 level 1-relation filter" should "work" in {
    server.query(query = """{albums(where:{ Artist: { is: { Name: { equals: "CompleteArtist" }}}}){AlbumId}}""", project = project).toString should be(
      """{"data":{"albums":[{"AlbumId":1}]}}""")
  }

  // MySql is case insensitive and Postgres case sensitive

  "MySql 1 level m-relation filter" should "work for `every`, `some` and `none`" taggedAs (IgnorePostgres, IgnoreMongo) in {
    server.query(query = """{artists(where:{Albums: { some: { Title: { startsWith: "album" }}}}){ Name }}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { some: {Title: { startsWith: "t" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}""")

    server.query(query = """{artists(where:{Albums: { every: { Title: { contains: "album" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { every: { Title: { not: { contains: "the" }}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { none: {Title: { contains: "the" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { none:{Title: { contains: "album" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithoutAlbums"}]}}""")
  }

  "Postgres 1 level m-relation filter" should "work for `some`" taggedAs (IgnoreMySql) in {
    server
      .query(query = """{artists(where:{Albums: { some: {Title: { startsWith: "Album" }}}}, orderBy: { id: asc }){Name}}""", project = project)
      .toString should be("""{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { some:{Title: { startsWith: "T" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}""")
  }

  "PostGres 1 level m-relation filter" should "work for every" taggedAs (IgnoreMySql, IgnoreMongo) in {
    server.query(query = """{artists(where:{Albums: { every:{Title: { contains: "Album" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { every:{Title: { not: { contains: "The" }}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")
  }

  "PostGres 1 level m-relation filter" should "work for `none`" taggedAs (IgnoreMySql, IgnoreMongo) in {
    server.query(query = """{artists(where:{Albums: { none:{Title: { contains: "The" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { none:{Title: { contains: "Album" }}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithoutAlbums"}]}}""")
  }

  "2 level m-relation filter" should "work for some/some" in {
    server
      .query(query = """{artists(where:{Albums: { some: { Tracks: { some: {Milliseconds: { lte: 9000 }}}}}}, orderBy: {Name: asc}){Name}}""", project = project)
      .toString should be("""{"data":{"artists":[{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{Albums: { some:{Tracks: { some: {Bytes: { equals: 512 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"}]}}""")
  }

  "2 level m-relation filter" should "work for every, some and none" taggedAs (IgnoreMongo) in {
    // some|every
    server.query(query = """{artists(where:{Albums: { some:{ Tracks: { every: { UnitPrice: { gt: 2.50 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server
      .query(query = """{artists(where:{ Albums: { some:{ Tracks: { every: { Milliseconds: { gt: 9000 }}}}}}){Name}}""", project = project)
      .toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    // some|none
    server
      .query(query = """{artists(where:{ Albums: { some:{ Tracks: { none: { Milliseconds: { lte: 9000 }}}}}}){Name}}""", project = project)
      .toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server.query(query = """{artists(where:{ Albums: { some:{ Tracks: { none: { UnitPrice: { lt: 2.0 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    // every|some
    server.query(query = """{artists(where:{ Albums: { every: { Tracks: { some: { Bytes: { lt: 1000 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    server
      .query(query = """{artists(where: { Albums: { every: { Tracks: { some: { Composer: {equals: "Composer3" }}}}}}){Name}}""", project = project)
      .toString should be("""{"data":{"artists":[{"Name":"ArtistWithoutAlbums"}]}}""")

    // every|every
    server
      .query(query = """{artists(where: { Albums: { every: { Tracks: { every: { Bytes: { lte: 10000 }}}}}}){Name}}""", project = project)
      .toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"}]}}""")

    server
      .query(query = """{artists(where: { Albums: { every: { Tracks: { every: { TrackId: { in: [4,5,6,7] }}}}}}){Name}}""", project = project)
      .toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtistWith2Albums"}]}}""")

    // every|none
    server.query(query = """{artists(where: { Albums: { every: { Tracks: { none: { UnitPrice: { lte: 1 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"}]}}""")

    server
      .query(query = """{artists(where: { Albums: { every: { Tracks: { none: { Composer: { equals: "Composer2" }}}}}}){Name}}""", project = project)
      .toString should be("""{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}""")

    // none|some
    server.query(query = """{artists(where: { Albums: { none: { Tracks: { some: { UnitPrice: { lt: 1 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"},{"Name":"CompleteArtist2"}]}}""")

    server
      .query(query = """{artists(where: { Albums: { none: { Tracks: { some: { Composer: { equals: "Composer2" }}}}}}){Name}}""", project = project)
      .toString should be("""{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"ArtistWithOneAlbumWithoutTracks"}]}}""")

    // none|every
    server.query(query = """{artists(where: { Albums: { none: { Tracks: { every: { UnitPrice: { gte: 5 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"}]}}""")

    server
      .query(query = """{artists(where: { Albums: { none: { Tracks: { every: { Name: { startsWith: "Track" }}}}}}){Name}}""", project = project)
      .toString should be("""{"data":{"artists":[{"Name":"ArtistWithoutAlbums"}]}}""")

    // none|none
    server.query(query = """{artists(where: { Albums: { none: { Tracks: { none: { Bytes: { lt: 100 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"}]}}""")

    server.query(query = """{artists(where: { Albums: { none: { Tracks: { none: { Bytes: { gte: 100 }}}}}}){Name}}""", project = project).toString should be(
      """{"data":{"artists":[{"Name":"CompleteArtist"},{"Name":"ArtistWithoutAlbums"},{"Name":"CompleteArtist2"},{"Name":"CompleteArtistWith2Albums"}]}}""")
  }

  "2 level m-relation filters that have subfilters that are connected with an implicit AND" should "work for `some`" in {
    server
      .query(
        query =
          """{albums(where: { Tracks: { some: { MediaType: { is: { Name: { equals: "MediaType1" }}}, Genre: { is: { Name: { equals: "Genre1" }}}}}}, orderBy: { id: asc }){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

  }

  "2 level m-relation filters that have subfilters that are connected with an implicit AND" should "work for `every`" taggedAs (IgnoreMongo) in {
    server
      .query(
        query =
          """{albums(where: { Tracks: { every: { MediaType: { is: { Name: { equals: "MediaType1" }}}, Genre: { is: { Name: { equals: "Genre1" }}}}}}){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album4"}]}}""")

  }

  "2 level m-relation filters that have subfilters that are connected with an explicit AND" should "work for `some`" in {
    server
      .query(
        query =
          """{albums(where: { Tracks: { some: { AND:[ { MediaType: { is: { Name: { equals: "MediaType1" }}}}, { Genre: { is: { Name: { equals: "Genre1" }}}}]}}}, orderBy: { id: asc }){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

    server
      .query(query = """{albums(where: { Tracks: { some: { AND: [{ MediaType: { is: { Name: { equals: "MediaType2" }}}}]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"Album3"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { some: { AND:[] }}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

  }

  "2 level m-relation filters that have subfilters that are connected with an explicit AND" should "work for `every`" taggedAs (IgnoreMongo) in {
    server
      .query(
        query =
          """{albums(where: { Tracks: { every: { AND: [{ MediaType: { is: { Name: { equals: "MediaType1" }}}}, { Genre: { is: { Name: { equals: "Genre1" }}}}]}}}){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album4"}]}}""")

    server
      .query(query = """{albums(where: { Tracks: { every: { AND: [{ MediaType: { is: { Name: { equals: "MediaType2" }}}}]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"TheAlbumWithoutTracks"}]}}""")

    server
      .query(query = """{albums(where: { Tracks: { every:{ AND: []}}}){Title}}""", project = project)
      .toString should be(
      """{"data":{"albums":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}""")
  }

  "2 level m-relation filters that have subfilters that are connected with an explicit OR" should "work" taggedAs (IgnoreMongo) in {
    server
      .query(
        query =
          """{albums(where:{Tracks: { some:{OR:[{MediaType: {is: {Name: { equals: "MediaType1" }}}}, { Genre: { is: { Name: { equals: "Genre2" }}}}]}}}){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

    server
      .query(
        query =
          """{albums(where:{Tracks: { every:{OR:[{MediaType: { is: {Name: { equals: "MediaType1"}}}},{Genre: { is: {Name: { equals: "Genre2"}}}}]}}}){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { some:{OR:[{MediaType: { is: {Name: { equals: "MediaType2"}}}}]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"Album3"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { every:{OR:[{MediaType: { is: {Name: { equals: "MediaType2"}}}}]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"TheAlbumWithoutTracks"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { some:{OR:[]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[]}}""")

    server
      .query(query = """{albums(where:{Tracks: { every:{OR:[]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"TheAlbumWithoutTracks"}]}}""")
  }

  "2 level m-relation filters that have subfilters that are connected with an explicit NOT" should "work" taggedAs (IgnoreMongo) in {
    server
      .query(
        query =
          """{albums(where:{Tracks: { some:{NOT:[{MediaType: { is: {Name: { equals: "MediaType1"}}}},{Genre: { is: { Name: { equals: "Genre1"}}}}]}}}, orderBy: { Title: asc}){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"Album3"},{"Title":"Album5"}]}}""")

    server
      .query(
        query =
          """{albums(where:{Tracks: { every:{NOT:[{MediaType: { is: {Name: { equals: "MediaType1"}}}},{Genre: { is: {Name: { equals: "Genre1"}}}}]}}}){Title}}""",
        project = project
      )
      .toString should be("""{"data":{"albums":[{"Title":"TheAlbumWithoutTracks"},{"Title":"Album3"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { some:{NOT:[{MediaType: { is: {Name: { equals: "MediaType2"}}}}]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { every:{NOT:[{MediaType: { is: {Name: { equals: "MediaType2"}}}}]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { some:{NOT:[]}}}){Title}}""", project = project)
      .toString should be("""{"data":{"albums":[{"Title":"Album1"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}""")

    server
      .query(query = """{albums(where:{Tracks: { every:{NOT:[]}}}){Title}}""", project = project)
      .toString should be(
      """{"data":{"albums":[{"Title":"Album1"},{"Title":"TheAlbumWithoutTracks"},{"Title":"Album3"},{"Title":"Album4"},{"Title":"Album5"}]}}""")
  }

  "3 level filters" should "work" in {
    val result = server.query(
      """{
        |  genres(
        |    where: {
        |      Tracks: {
        |        some: { Album: { is: { Artist: { is: { ArtistId: { equals: 1 } } } } } }
        |      }
        |    }
        |  ) {
        |    GenreId
        |  }
        |}
        |
      """.stripMargin,
      project
    )

    result.pathAsJsValue("data") shouldBe """{"genres":[{"GenreId":1}]}""".parseJson
  }

  "3 level filters with a relation filter and a scalar filter" should "work" in {
    val result = server.query(
      """{
        |  artists(where: {
        |      Albums: {
        |        some: {
        |          Tracks: { some: {
        |            Genre: { is: { Name: { equals: "Genre1" }}}
        |            TrackId: { equals: 1 }
        |          }
        |        }
        |      }
        |    }
        |  }) {
        |    ArtistId
        |  }
        |}
      """.stripMargin,
      project
    )
    result.pathAsJsValue("data") shouldBe """{"artists":[{"ArtistId":1}]}""".parseJson
  }

  "an empty `none` filter" should "return all nodes that do have an empty relation" taggedAs (IgnoreMongo) in {
    server.query(query = """{genres(where:{Tracks: { none:{}}}){Name}}""", project = project).toString should be(
      """{"data":{"genres":[{"Name":"GenreThatIsNotUsed"}]}}""")
  }

  "an empty `some` filter" should "return all nodes that do have a non-empty relation" in {
    server.query(query = """{genres(where:{Tracks: { some:{} }}){Name}}""", project = project).toString should be(
      """{"data":{"genres":[{"Name":"Genre1"},{"Name":"Genre2"},{"Name":"Genre3"}]}}""")
  }
}
