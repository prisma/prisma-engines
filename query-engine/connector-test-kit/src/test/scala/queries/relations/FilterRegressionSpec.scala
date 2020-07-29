package queries.relations

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class FilterRegressionSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Querying a 1:1 self relation with nulls" should "work on both sides" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Location {
         |  id        Int     @id
         |  name      String?
         |  companyId Int?
         |  company   Company?  @relation(fields: [companyId], references: [id])
         |}
         |
         |model Company {
         |  id        Int     @id
         |  name      String?
         |  locations Location[]
         |}
       """.stripMargin
    }
    database.setup(project)

    server.query("""mutation {createLocation(data: { id: 310, name: "A"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 311, name: "A"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 314, name: "A"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 312, name: "B"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 317, name: "B"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 313, name: "C"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 315, name: "C"}){id}}""", project)
    server.query("""mutation {createLocation(data: { id: 316, name: "D"}){id}}""", project)

    server.query(
      """
        |mutation {
        |  createCompany(data: { id: 134, name: "1", locations: {connect:[{id: 310},{id: 312},{id: 313} ]}}){
        |    id
        |    name
        |    locations {id}
        |  }
        |}
      """,
      project
    )

    server.query(
      """
        |mutation {
        |  createCompany(data: { id: 135, name: "2", locations: {connect:[{id: 311},{id: 314} ]}}){
        |    id
        |    name
        |    locations {id}
        |  }
        |}
      """,
      project
    )

    server.query(
      """
        |mutation {
        |  createCompany(data: { id: 136, name: "3", locations: {connect:[{id: 315},{id: 317}]}}){
        |    id
        |    name
        |    locations {id}
        |  }
        |}
      """,
      project
    )

    val find_1 = server.query("""query {companies(where: {locations_none: {name: "D"}}){id}}""", project)

    find_1.toString should be("{\"data\":{\"companies\":[{\"id\":1},{\"id\":2},{\"id\":3}]}}")
  }
}
