package queries.relations

import org.scalatest.{FlatSpec, Matchers}
import util.{ApiSpecBase, ProjectDsl}

class DuplicateColumnRegressionSpec extends FlatSpec with Matchers with ApiSpecBase {
  "Querying a scalarfield that would already be included since it backs a relationfield" should "only request the underlying column once" in {
    val project = ProjectDsl.fromString {
      s"""
         |model Transcriber {
         |  id               String                       @id
         |  competencies     TranscriberCompetency[]
         |}
         |
         |model TranscriberCompetency {
         |  id            String      @id
         |  transcriber   Transcriber @relation(fields: [transcriberId], references: [id])
         |  transcriberId String
         |  competency    Competency  @relation(fields: [competencyId], references: [id])
         |  competencyId  String
         |  @@unique([transcriberId, competencyId])
         |}
         |
         |model Competency {
         |  id                          String                       @id
         |  transcriberCompetencies     TranscriberCompetency[]
         |}
       """.stripMargin
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createTranscriber(data: { id: "one", competencies: { create: { id: "one_trans", competency: {create:{ id: "one_comp"}}} } }){
        |    id
        |    competencies{
        |    id
        |    transcriberId
        |    competency
        |     {id}
        |    }
        |
        |  }
        |}
      """.stripMargin,
      project
    )

    result.toString should be(
      "{\"data\":{\"createTranscriber\":{\"id\":\"one\",\"competencies\":[{\"id\":\"one_trans\",\"transcriberId\":\"one\",\"competency\":{\"id\":\"one_comp\"}}]}}}")
  }
}
