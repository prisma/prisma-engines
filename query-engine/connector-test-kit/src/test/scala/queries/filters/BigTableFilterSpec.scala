package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class BigTableFilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  val project = ProjectDsl.fromString { """model User {
                                          |  id       Int @id
                                          |  artist   Artist?
                                          |}
                                          |
                                          |model Artist {
                                          |  id Int @id
                                          |  user User[]
                                          |}
                                          |""" }


  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)

    // add data
    server.query("""mutation {createArtist(data: {id: 1}){id}}""", project = project)

    val queries = (1 to 35).map(i => s"""mutation {createUser(data: {id: $i}){id}}""")

    server.batch(queries, project)

  }

  "paginated relation query" should "work" in {
    val expected = """{"data":{"users":[{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null},{"artist":null}]}}"""
    server.query("""query {users(first: 35, skip: 0, orderBy: id_ASC){artist{id}}}""", project).toString() should be(expected)
  }
}
