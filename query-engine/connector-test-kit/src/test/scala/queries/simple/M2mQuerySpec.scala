package queries.simple

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.RelationLinkTableCapability
import util._

class M2mQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(RelationLinkTableCapability) // Is this correct?

  val project: Project = ProjectDsl.fromString { """
                                                   |model Blog {
                                                   |  slug       String @id
                                                   |  title      String
                                                   |  content    String
                                                   |  categories Category[]
                                                   |}
                                                   |
                                                   |model Category {
                                                   |  name  String @id
                                                   |  blogs Blog[]
                                                   |}
                                                   """.stripMargin }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
  }

  "Queries" should "only fetch associated records" in {
    server
      .query(
        s"""mutation {
           |  createBlog(data: {
           |    slug: "Slug1"
           |    title: "TestTitle1"
           |    content: "TestContent1"
           |    categories: {
           |      create: [{
           |        name: "Cat1"
           |      },{
           |        name: "Cat2"
           |      }]
           |    }
           |  }) {
           |    slug
           |    title
           |  }
           |}""".stripMargin,
        project
      )

    val result = server
      .query(
        s"""mutation {
           |  createBlog(data: {
           |    slug: "Slug2"
           |    title: "TestTitle2"
           |    content: "TestContent2"
           |    categories: {
           |      create: [{
           |        name: "Cat3"
           |      },{
           |        name: "Cat4"
           |      }]
           |    }
           |  }) {
           |    slug
           |    title
           |    categories {
           |      name
           |    }
           |  }
           |}""".stripMargin,
        project
      )

    result.pathAsSeq("data.createBlog.categories").length should equal(2)
  }
}
