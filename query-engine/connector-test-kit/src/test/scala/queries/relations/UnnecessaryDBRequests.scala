package queries.relations

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class UnnecessaryDBRequests extends FlatSpec with Matchers with ApiSpecBase {
  "One to Many relations" should "not create unnecessary requests" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Top {
         |  id            String  @id
         |  middle_id     String?
         |  middle        Middle? @relation(fields: [middle_id], references: [id])
         |}
         |
         |model Middle {
         |  id            String  @id
         |  bottom_id     String?
         |  bottom        Bottom? @relation(fields: [bottom_id], references: [id])
         |}
         |
         |model Bottom {
         |  id            String  @id
         |}
       """.stripMargin
    }
    database.setup(project)

    server.query("""
        |mutation {
        |  createTop(data: { id: "lonely_top" }){
        |    id
        |  }
        |}
      """,
                 project)

    server.query(
      """
                   |mutation {
                   |  createTop(data: { 
                   |    id: "family_top"
                   |    middle: { create:{
                   |      id: "middle"
                   |      bottom: { create:{
                   |        id: "bottom"
                   |      }}
                   |    }
                   |    }
                   |   }){
                   | id,
                   | middle{
                   |    id
                   |    bottom {
                   |      id
                   |    }
                   | }
                   |  }
                   |}
      """,
      project
    )

    //lonely top
    // todo
    //  Start:    3 request
    //  Current:  2 request
    //  Goal:     1 request
    val lonely = server.query(
      """
                   |query {
                   |  tops(where: { id: { equals: "lonely_top" }}){
                   |     id,
                   |  middle{
                   |     id
                   |     bottom {
                   |       id
                   |     }
                   |  }
                   |  }
                   |}
      """,
      project
    )

    lonely.toString() should be("{\"data\":{\"tops\":[{\"id\":\"lonely_top\",\"middle\":null}]}}")

    //family top
    val family = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "family_top" }}){
        |     id,
        |  middle{
        |     id
        |     bottom {
        |       id
        |     }
        |  }
        |  }
        |}
      """,
      project
    )

    family.toString() should be("{\"data\":{\"tops\":[{\"id\":\"family_top\",\"middle\":{\"id\":\"middle\",\"bottom\":{\"id\":\"bottom\"}}}]}}")

  }

  "Many to Many relations" should "not create unnecessary requests" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Top {
         |  id            String  @id
         |  middle        Middle[]
         |}
         |
         |model Middle {
         |  id            String  @id
         |  top           Top[]
         |  bottom        Bottom[]
         |}
         |
         |model Bottom {
         |  id            String  @id
         |  middle        Middle[]
         |}
       """.stripMargin
    }
    database.setup(project)

    server.query(
      """
                   |mutation {
                   |  createTop(data: { id: "lonely_top" }){
                   |    id
                   |  }
                   |}
      """,
      project
    )

    server.query(
      """
        |mutation {
        |  createTop(data: { 
        |    id: "family_top"
        |    middle: { create:{
        |      id: "middle"
        |      bottom: { create:{
        |        id: "bottom"
        |      }}
        |    }
        |    }
        |   }){
        | id,
        | middle{
        |    id
        |    bottom {
        |      id
        |    }
        | }
        |  }
        |}
      """,
      project
    )

    //lonely top
    // todo
    //  Start:    5 request
    //  Current:  2 request
    //  Goal:     2 request
    val lonely = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "lonely_top" }}){
        |     id,
        |  middle{
        |     id
        |     bottom {
        |       id
        |     }
        |  }
        |  }
        |}
      """,
      project
    )

    lonely.toString() should be("{\"data\":{\"tops\":[{\"id\":\"lonely_top\",\"middle\":[]}]}}")

    //family top
    val family = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "family_top" }}){
        |     id,
        |  middle{
        |     id
        |     bottom {
        |       id
        |     }
        |  }
        |  }
        |}
      """,
      project
    )

    family.toString() should be("{\"data\":{\"tops\":[{\"id\":\"family_top\",\"middle\":[{\"id\":\"middle\",\"bottom\":[{\"id\":\"bottom\"}]}]}]}}")

  }

  //Todo
  // what about 1:1
  // the unnecessary second level request for one2many is still generated
  // relation{id} extra request for id
  // fetching related id when resolving m2m (its already in the relation table)
  //
}
