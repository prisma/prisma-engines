package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.{DocumentConnectorTag, RelationalConnectorTag}
import util._

class SelfRelationFilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val project = ProjectDsl.fromString {
    connectorTag match {
      case _: RelationalConnectorTag =>
        """model Human {
          |  id         String  @id @default(cuid())
          |  name       String
          |  wife_id    String?
          |  mother_id  String?
          |  father_id  String?
          |  singer_id  String?
          |  title_id   String?
          |
          |  husband       Human? @relation(name: "Marriage")
          |  wife          Human? @relation(name: "Marriage", fields: [wife_id], references: [id])
          |  mother        Human? @relation(name:"Cuckoo", fields: [mother_id], references: [id])
          |  father        Human? @relation(name:"Offspring", fields: [father_id], references: [id])
          |  singer        Human? @relation(name:"Team", fields: [singer_id], references: [id])
          |  title         Song?  @relation(fields: [title_id], references: [id])
          |
          |  daughters     Human[] @relation(name:"Offspring")
          |  stepdaughters Human[] @relation(name:"Cuckoo")
          |  fans          Human[] @relation(name:"Admirers")
          |  rockstars     Human[] @relation(name:"Admirers")
          |  bandmembers   Human[] @relation(name:"Team")
          |}
          |
          |model Song{
          |   id      String @id @default(cuid())
          |   title   String
          |   creator Human?
          |}""".stripMargin

      case _: DocumentConnectorTag =>
        """model Human{
          |   id            String @id @default(cuid())
          |   name          String
          |   wife          Human?  @relation(name: "Marriage", references: [id])
          |   husband       Human?  @relation(name: "Marriage")
          |   daughters     Human[] @relation(name:"Offspring")
          |   father        Human?  @relation(name:"Offspring", references: [id])
          |   stepdaughters Human[] @relation(name:"Cuckoo")
          |   mother        Human?  @relation(name:"Cuckoo", references: [id])
          |   fans          Human[] @relation(name:"Admirers", references: [id])
          |   rockstars     Human[] @relation(name:"Admirers")
          |   singer        Human?  @relation(name:"Team", references: [id])
          |   bandmembers   Human[] @relation(name:"Team")
          |   title         Song?   @relation(references: [id])
          |}
          |
          |model Song{
          |   id      String @id @default(cuid())
          |   title   String
          |   creator Human?
          |}""".stripMargin
    }
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    setupRockRelations
  }

  "Filter Queries along self relations" should "succeed with one level " in {
    val filterKurt =
      s"""
         |query {
         |  songs(
         |    where: {
         |      creator: {
         |        is: {
         |          name: { equals: "kurt" }
         |        }
         |      }
         |    }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterKurt, project, dataContains = "{\"songs\":[{\"title\":\"My Girl\"},{\"title\":\"Gasag\"}]}")
  }

  "Filter Queries along self relations" should "succeed with two levels" in {
    val filterFrances =
      s"""
         |query {
         |  songs(
         |    where: {
         |      creator: {
         |        is: {
         |          daughters: {
         |            some: {
         |              name: { equals: "frances" }
         |            }
         |          }
         |        }
         |      }
         |    }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterFrances, project, dataContains = "{\"songs\":[{\"title\":\"My Girl\"}]}")
  }

  "Filter Queries along OneToOne self relations" should "succeed with two levels 2" in {
    val filterWife =
      s"""
         |query {
         |  songs(
         |    where: {
         |      creator: {
         |        is: {
         |          wife: {
         |            is: {
         |              name: { equals: "yoko" }
         |            }
         |          }
         |        }
         |      }
         |    }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterWife, project, dataContains = "{\"songs\":[{\"title\":\"Imagine\"}]}")
  }

  "Filter Queries along OneToOne self relations" should "succeed with null filter" in {
    val filterWifeNull =
      s"""
         |query {
         |  songs(where: { creator: { is: { wife: { is: null }}}}) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterWifeNull, project, dataContains = "{\"songs\":[{\"title\":\"Bicycle\"},{\"title\":\"Gasag\"}]}")
  }

  "Filter Queries along OneToOne self relations" should "succeed with {} filter" in {
    val filterWifeNull =
      s"""
         |query {
         |  songs(
         |    where: {
         |      creator: {
         |        is: {
         |          wife: { is: {} }
         |        }
         |      }
         |    }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterWifeNull, project, dataContains = "{\"songs\":[{\"title\":\"My Girl\"},{\"title\":\"Imagine\"}]}")
  }

  "Filter Queries along OneToMany self relations" should "fail with null filter" taggedAs (IgnoreMongo) in {
    val filterDaughterNull =
      s"""
         | query {
         |   songs(
         |     where: {
         |       creator: {
         |         is: {
         |           daughters: { none: null }
         |         }
         |       }
         |     }
         |   ) {
         |     title
         |  }
         |}
       """.stripMargin

    server.queryThatMustFail(
      filterDaughterNull,
      project,
      errorCode = 2012,
      errorContains =
        "Missing a required value at `Query.songs.where.SongWhereInput.creator.HumanRelationFilter.is.HumanWhereInput.daughters.HumanListRelationFilter.none`"
    )
  }

  "Filter Queries along OneToMany self relations" should "succeed with empty filter {}" in {
    val filterDaughter =
      s"""
         |query {
         |  songs(
         |    where: {
         |      creator: {
         |        is: {
         |          daughters: { some: {} }
         |        }
         |      }
         |    }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterDaughter, project, dataContains = "{\"songs\":[{\"title\":\"My Girl\"}]}")
  }

  // ManyToMany

  "Filter Queries along ManyToMany self relations" should "succeed with valid filter `some`" in {
    val filterGroupies =
      s"""
         |query {
         |  songs(
         |    where: { creator: { is: { fans: { some: { name: { equals: "groupie1" }}}}}}
         |    orderBy: { id: asc }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterGroupies, project, dataContains = "{\"songs\":[{\"title\":\"My Girl\"},{\"title\":\"Imagine\"}]}")
  }

  "Filter Queries along ManyToMany self relations" should "succeed with valid filter `none`" taggedAs (IgnoreMongo) in {
    val filterGroupies =
      s"""
         |query {
         |  songs(where: { creator: { is: { fans: { none: { name: { equals: "groupie1" }}}}}}) {
         |    title
         |  }
         |}
         |
       """.stripMargin

    server.query(filterGroupies, project, dataContains = "{\"songs\":[{\"title\":\"Bicycle\"},{\"title\":\"Gasag\"}]}")
  }

  "Filter Queries along ManyToMany self relations" should "succeed with valid filter `every`" taggedAs (IgnoreMongo) in {

    val filterGroupies =
      s"""
         |query {
         |  songs(where: { creator: { is: { fans: { every: { name: { equals: "groupie1" }}}}}}) {
         |    title
         |  }
         |}
       """.stripMargin

    server.query(filterGroupies, project, dataContains = "{\"songs\":[{\"title\":\"Imagine\"},{\"title\":\"Bicycle\"},{\"title\":\"Gasag\"}]}")
  }

  "Filter Queries along ManyToMany self relations" should "give an error with null" taggedAs (IgnoreMongo) in {
    val filterGroupies =
      s"""
         |query {
         |  songs(
         |    where: { creator: { is: { fans: { every: { fans: { some: null } } } } } }
         |  ) {
         |    title
         |  }
         |}
       """.stripMargin

    server.queryThatMustFail(
      filterGroupies,
      project,
      errorCode = 2012,
      errorContains =
        """Missing a required value at `Query.songs.where.SongWhereInput.creator.HumanRelationFilter.is.HumanWhereInput.fans.HumanListRelationFilter.every.HumanWhereInput.fans.HumanListRelationFilter.some`"""
    )
  }

  "Filter Queries along ManyToMany self relations" should "succeed with {} filter `some`" in {
    val filterGroupies =
      s"""
         |query {
         |  songs(where: { creator: { is: { fans: { some: {} } } } }) {
         |    title
         |  }
         |}
         |
       """.stripMargin

    server.query(filterGroupies, project, dataContains = "{\"songs\":[{\"title\":\"My Girl\"},{\"title\":\"Imagine\"}]}")
  }

  "Filter Queries along ManyToMany self relations" should "succeed with {} filter `none`" taggedAs (IgnoreMongo) in {
    val filterGroupies =
      s"""
         |query {
         |  humans(where: { fans: { none: {} } }, orderBy: { id: asc }) {
         |    name
         |  }
         |}
       """.stripMargin

    server.query(
      filterGroupies,
      project,
      dataContains =
        "{\"humans\":[{\"name\":\"paul\"},{\"name\":\"dave\"},{\"name\":\"groupie1\"},{\"name\":\"groupie2\"},{\"name\":\"frances\"},{\"name\":\"courtney\"},{\"name\":\"yoko\"},{\"name\":\"freddy\"},{\"name\":\"kurt\"}]}"
    )
  }

  "Filter Queries along ManyToMany self relations" should "succeed with {} filter `every`" taggedAs (IgnoreMongo) in {
    val filterGroupies =
      s"""
         |query {
         |  humans(where: { fans: { every: {} } }, orderBy: { id: asc }) {
         |    name
         |  }
         |}
       """.stripMargin

    server.query(
      filterGroupies,
      project,
      dataContains =
        "{\"humans\":[{\"name\":\"paul\"},{\"name\":\"dave\"},{\"name\":\"groupie1\"},{\"name\":\"groupie2\"},{\"name\":\"frances\"},{\"name\":\"courtney\"},{\"name\":\"kurt\"},{\"name\":\"yoko\"},{\"name\":\"john\"},{\"name\":\"freddy\"},{\"name\":\"kurt\"}]}"
    )
  }

  // Many to one

  "Filter Queries along ManyToOne self relations" should "succeed valid filter" in {
    val filterSingers =
      s"""
         |query {
         |  humans(where: { singer: { is: { name: { equals: "kurt" } } } }) {
         |    name
         |  }
         |}
       """.stripMargin

    server.query(filterSingers, project, dataContains = "{\"humans\":[{\"name\":\"dave\"}]}")
  }

  "Filter Queries along ManyToOne self relations" should "succeed with {} filter" in {
    val filterSingers =
      s"""
         |query {
         |  humans(where: { singer: { is: {} } }, orderBy: { id: asc }) {
         |    name
         |  }
         |}
       """.stripMargin

    server.query(filterSingers, project, dataContains = "{\"humans\":[{\"name\":\"paul\"},{\"name\":\"dave\"}]}")
  }

  "Filter Queries along ManyToOne self relations" should "succeed with null filter" in {

    val filterSingers =
      s"""
         |query {
         |  humans(where: { singer: { is: null } }, orderBy: { id: asc }) {
         |    name
         |  }
         |}
       """.stripMargin

    server.query(
      filterSingers,
      project,
      dataContains =
        "{\"humans\":[{\"name\":\"groupie1\"},{\"name\":\"groupie2\"},{\"name\":\"frances\"},{\"name\":\"courtney\"},{\"name\":\"kurt\"},{\"name\":\"yoko\"},{\"name\":\"john\"},{\"name\":\"freddy\"},{\"name\":\"kurt\"}]}"
    )
  }

  override def beforeEach() = {} // do not delete dbs on each run

  private def setupRockRelations = {

    val paul = server.query("""mutation{createHuman(data:{name: "paul"}){id}}""", project).pathAsString("data.createHuman.id")

    val dave = server.query("""mutation{createHuman(data:{name: "dave"}){id}}""", project).pathAsString("data.createHuman.id")

    val groupie1 = server.query("""mutation{createHuman(data:{name: "groupie1"}){id}}""", project).pathAsString("data.createHuman.id")

    val groupie2 = server.query("""mutation{createHuman(data:{name: "groupie2"}){id}}""", project).pathAsString("data.createHuman.id")

    val frances = server.query("""mutation{createHuman(data:{name: "frances"}){id}}""", project).pathAsString("data.createHuman.id")

    val courtney = server
      .query(s"""mutation{createHuman(data:{name: "courtney",stepdaughters: {connect: [{id: "$frances"}]}}){id}}""", project)
      .pathAsString("data.createHuman.id")

    val kurtc = server
      .query(
        s"""mutation{createHuman(data:{name: "kurt",
         |wife: {connect: { id: "$courtney"}},
         |daughters: {connect: [{ id: "$frances"}]},
         |fans: {connect: [{id: "$groupie1"}, {id: "$groupie2"}]},
         |bandmembers: {connect:[{id: "$dave"}]}}){id}}""".stripMargin,
        project
      )
      .pathAsString("data.createHuman.id")

    val mygirl =
      server.query(s"""mutation{createSong(data:{title: "My Girl", creator: {connect: { id: "$kurtc"}}}){id}}""", project).pathAsString("data.createSong.id")

    val yoko = server.query(s"""mutation{createHuman(data:{name: "yoko"}){id}}""", project).pathAsString("data.createHuman.id")

    val john = server
      .query(
        s"""mutation{createHuman(data:{name: "john",
         |wife: {connect: { id: "$yoko"}}
         |fans: {connect:[{id: "$groupie1"}]}
         |bandmembers: {connect: [{id: "$paul"}]}}){id}}""".stripMargin,
        project
      )
      .pathAsString("data.createHuman.id")

    val imagine =
      server.query(s"""mutation{createSong(data:{title: "Imagine", creator: {connect: { id: "$john"}}}){id}}""", project).pathAsString("data.createSong.id")

    val freddy = server.query(s"""mutation{createHuman(data:{name: "freddy"}){id}}""", project).pathAsString("data.createHuman.id")

    val bicycle =
      server.query(s"""mutation{createSong(data:{title: "Bicycle", creator: {connect: { id: "$freddy"}}}){id}}""", project).pathAsString("data.createSong.id")

    val kurtk = server.query(s"""mutation{createHuman(data:{name: "kurt"}){id}}""", project).pathAsString("data.createHuman.id")

    val gasag =
      server.query(s"""mutation{createSong(data:{title: "Gasag", creator: {connect: { id: "$kurtk"}}}){id}}""", project).pathAsString("data.createSong.id")
  }
}
