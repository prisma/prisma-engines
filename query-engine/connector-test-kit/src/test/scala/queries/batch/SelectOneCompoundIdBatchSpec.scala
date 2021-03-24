package queries.batch

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class SelectOneCompoundIdBatchSpec extends FlatSpec with Matchers with ApiSpecBase {

  val project = ProjectDsl.fromString {
    """model Artist {
      |  firstName String
      |  lastName  String
      |  
      |  @@unique([firstName, lastName])
      |}
      |"""
  }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)

    server.query(
      """mutation artists {createArtist(data:{
        |                         firstName: "Musti"
        |                         lastName: "Naukio"
        |        |}){firstName lastName}}""".stripMargin,
      project = project
    )

    server.query(
      """mutation artists {createArtist(data:{
        |                         firstName: "Naukio"
        |                         lastName: "Musti"
        |        |}){firstName lastName}}""".stripMargin,
      project = project
    )
  }

  "one successful query" should "work" in {
    server
      .batch(
        Seq("""query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}"""),
        transaction = false,
        project,
        legacy = false
      )
      .toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}}]}"""
    )
  }

  "two successful queries and one failing with same selection set" should "work" in {
    val queries = Seq(
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}""",
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}""",
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {firstName lastName}}""",
    )

    server.batch(queries, transaction = false, project, legacy = false).toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"""
    )
  }

  "two successful queries with selection set in a different order" should "work" in {
    val queries = Seq(
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}""",
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {lastName firstName}}""",
    )

    server.batch(queries, transaction = false, project, legacy = false).toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"""
    )
  }

  "two successful queries and one failing with different selection set" should "work" in {
    val queries = Seq(
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}""",
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {lastName}}""",
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Naukio",lastName:"Musti"}}) {firstName lastName}}""",
    )

    server.batch(queries, transaction = false, project, legacy = false).toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}},{"data":{"findUniqueArtist":{"firstName":"Naukio","lastName":"Musti"}}}]}"""
    )
  }

  "one singular failing query" should "work" in {

    server
      .batch(Seq("""query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {lastName}}"""),
             transaction = false,
             project,
             legacy = false)
      .toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":null}}]}"""
    )
  }

  "one singular failing query out of two" should "work" in {
    val queries = Seq(
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"Musti",lastName:"Naukio"}}) {firstName lastName}}""",
      """query {findUniqueArtist(where:{firstName_lastName:{firstName:"NO",lastName:"AVAIL"}}) {firstName lastName}}""",
    )

    server.batch(queries, transaction = false, project, legacy = false).toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Musti","lastName":"Naukio"}}},{"data":{"findUniqueArtist":null}}]}"""
    )
  }

  // https://github.com/prisma/prisma/issues/5941
  "input dates in two queries" should "not return nulls" in {
    val project = ProjectDsl.fromString {
      """model Artist {
        |  firstName String
        |  lastName  String
        |  birth     DateTime
        |
        |  @@unique([firstName, lastName, birth])
        |}
        |"""
    }

    database.setup(project)

    server.query(
      """mutation artists {createArtist(data:{
        |                         firstName: "Sponge"
        |                         lastName: "Bob"
        |                         birth: "1999-05-01T00:00:00.000Z"
        |        |}){firstName lastName birth}}""".stripMargin,
      project = project
    )

    val queries = Seq(
      """query {findUniqueArtist(where:{firstName_lastName_birth:{firstName:"Sponge",lastName:"Bob", birth: "1999-05-01T00:00:00.000Z"}}) {firstName lastName birth}}""",
      """query {findUniqueArtist(where:{firstName_lastName_birth:{firstName:"Sponge",lastName:"Bob", birth: "1999-05-01T00:00:00.000Z"}}) {firstName lastName birth}}""",
    )

    server.batch(queries, transaction = false, project, legacy = false).toString should be(
      """{"batchResult":[{"data":{"findUniqueArtist":{"firstName":"Sponge","lastName":"Bob","birth":"1999-05-01T00:00:00.000Z"}}},{"data":{"findUniqueArtist":{"firstName":"Sponge","lastName":"Bob","birth":"1999-05-01T00:00:00.000Z"}}}]}"""
    )
  }
}
