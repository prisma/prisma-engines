package queries.relations

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class UnnecessaryDBRequests extends FlatSpec with Matchers with ApiSpecBase {
  "One to Many relations" should "not create unnecessary roundtrips" in {
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

    //family top
    //  Start:    3 roundtrip
    //  Current:  3 roundtrip
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

    //lonely top
    //  Start:    3 roundtrip
    //  Current:  1 roundtrip
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

    //no top
    //  Start:    3 roundtrip
    //  Current:  1 roundtrip
    val no = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "does not exist" }}){
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

    no.toString() should be("{\"data\":{\"tops\":[]}}")

    //two levels
    //  Start:    3 roundtrip
    //  Current:  1 roundtrip
    val two_levels = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "family_top" }}){
        |     id,
        |     middle{
        |       id
        |   }
        |  }
        |}
      """,
      project
    )

    two_levels.toString() should be("{\"data\":{\"tops\":[{\"id\":\"family_top\",\"middle\":{\"id\":\"middle\"}}]}}")

  }

  "Many to Many relations" should "not create unnecessary round trips" in {
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

    //family top
    //  Start:    5 roundtrip
    //  Current:  5 roundtrip
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

    //lonely top
    //  Start:    5 roundtrip
    //  Current:  2 roundtrip
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

    //no top
    //  Start:    5 roundtrip
    //  Current:  1 roundtrip
    val no = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "does not exist" }}){
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

    no.toString() should be("{\"data\":{\"tops\":[]}}")

    //two levels
    //  Start:    3 roundtrip
    //  Current:  3 roundtrip
    //  Goal:     2 roundtrip
    val two_levels = server.query(
      """
        |query {
        |  tops(where: { id: { equals: "family_top" }}){
        |     id,
        |     middle{
        |       id
        |   }
        |  }
        |}
      """,
      project
    )

    two_levels.toString() should be("{\"data\":{\"tops\":[{\"id\":\"family_top\",\"middle\":[{\"id\":\"middle\"}]}]}}")
  }
}
