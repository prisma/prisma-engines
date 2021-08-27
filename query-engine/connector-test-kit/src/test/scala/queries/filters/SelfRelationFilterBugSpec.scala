package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util.ConnectorTag.{DocumentConnectorTag, RelationalConnectorTag}
import util._

// RS: Ported
class SelfRelationFilterBugSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val project = ProjectDsl.fromString {
    connectorTag match {
      case _: RelationalConnectorTag =>
        """model Category {
          |  id        String    @id @default(cuid())
          |  name      String
          |  parent_id String?
          |
          |  parent   Category? @relation(name: "C", fields: [parent_id], references: [id])
          |  opposite Category? @relation(name: "C")
          |}"""

      case _: DocumentConnectorTag =>
        """model Category {
          |  id        String    @id @default(cuid())
          |  name      String
          |  parent_id String?
          |
          |  parent   Category? @relation(name: "C", fields: [parent_id], references: [id])
          |  opposite Category? @relation(name: "C")
          |}"""
    }
  }

  override protected def beforeEach(): Unit = {
    super.beforeEach()
    database.setup(project)
    val id = server
      .query("""mutation{createCategory(data:{name: "Sub", parent: {create:{ name: "Root"}} }){parent{id}}}""", project)
      .pathAsString("data.createCategory.parent.id")
  }

  "Getting all categories" should "succeed" taggedAs (IgnoreMsSql) in {
    val allCategories =
      s"""{
         |  allCategories: categories(orderBy: { id: asc }) {
         |    name
         |    parent {
         |      name
         |    }
         |  }
         |}"""

    val res1 = server.query(allCategories, project).toString
    res1 should be("""{"data":{"allCategories":[{"name":"Sub","parent":{"name":"Root"}},{"name":"Root","parent":null}]}}""")
  }

  "Getting root categories categories" should "succeed" taggedAs (IgnoreMsSql) in {
    val rootCategories =
      s"""{
         |  allRootCategories: categories(where: { parent: { is: null }}) {
         |    name
         |    parent {
         |      name
         |    }
         |  }
         |}"""

    val res2 = server.query(rootCategories, project).toString
    res2 should be("""{"data":{"allRootCategories":[{"name":"Root","parent":null}]}}""")
  }

  "Getting subcategories with not" should "succeed" taggedAs (IgnoreMongo, IgnoreMsSql) in {
    val subCategories = s"""{
                               |  allSubCategories: categories(
                               |    where: { NOT: [{parent: { is: null }}] }
                               |  ) {
                               |    name
                               |    parent {
                               |      name
                               |    }
                               |  }
                               |}"""

    val res3 = server.query(subCategories, project).toString
    res3 should be("""{"data":{"allSubCategories":[{"name":"Sub","parent":{"name":"Root"}}]}}""")
  }

  "Getting subcategories with value" should "succeed" taggedAs (IgnoreMsSql) in {
    val subCategories2 = s"""{
                           |  allSubCategories2: categories(
                           |    where: { parent: { is: { name: { equals: "Root" }}}}
                           |  ) {
                           |    name
                           |    parent {
                           |      name
                           |    }
                           |  }
                           |}"""

    val res4 = server.query(subCategories2, project).toString
    res4 should be("""{"data":{"allSubCategories2":[{"name":"Sub","parent":{"name":"Root"}}]}}""")

  }

}
