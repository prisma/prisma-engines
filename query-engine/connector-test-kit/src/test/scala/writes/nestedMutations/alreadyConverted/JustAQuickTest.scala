package writes.nestedMutations.alreadyConverted

import org.scalatest.{Matchers, WordSpecLike}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class JustAQuickTest extends WordSpecLike with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "a P1! to C1! relation should be possible" in {
    val project = SchemaDsl.fromStringV11() {

      s"""
                model Parent {
                    p_1           String?
                    p_2           String?
                    children      Child []
                    non_unique    String?

                    @@id([p_1, p_2])
                }

                model Child {
                    c_1           String?
                    c_2           String?
                    parents       Parent
                    non_unique    String?

                    @@id([c_1, c_2])
                }
    """
    }
    database.setup(project)

    val res = server
      .query(
        """mutation {
          |  createParent(data: {
          |    p_1: "p_1"
          |    p_2: "p_2"
          |    non_unique: "test"
          |    children: {
          |      create: {
          |        c_1: "c_1"
          |        c_2: "c_2"
          |        non_unique: "test"
          |      }
          |    }
          |  }){
          |    p_1
          |    p_2
          |    children{
          |       c_1,
          |       c_2
          |    }
          |  }
          |}""",
        project
      )

    res.toString should be("""{"data":{"createParent":{"p_1":"p_1","p_2":"p_2","children":[{"c_1":"c_1","c_2":"c_2"}]}}}""")


    val res2 = server
      .query(
        """mutation {
          |  createParent(data: {
          |    p_1: "p_3"
          |    p_2: "p_4"
          |    non_unique: "test"
          |    children: {
          |      create: {
          |        c_1: "c_3"
          |        c_2: "c_4"
          |        non_unique: "test"
          |      }
          |    }
          |  }){
          |    p_1
          |    p_2
          |    children{
          |       c_1,
          |       c_2
          |    }
          |  }
          |}""",
        project
      )

    res2.toString should be("""{"data":{"createParent":{"p_1":"p_3","p_2":"p_4","children":[{"c_1":"c_3","c_2":"c_4"}]}}}""")


    val res3 = server
      .query(
        """query {
          |  parents{
          |    p_1
          |    p_2
          |    children{
          |       c_1,
          |       c_2
          |    }
          |  }
          |}""",
        project
      )

    res3.toString should be("""{"data":{"parents":[{"p_1":"p_1","p_2":"p_2","children":[{"c_1":"c_1","c_2":"c_2"}]},{"p_1":"p_3","p_2":"p_4","children":[{"c_1":"c_3","c_2":"c_4"}]}]}}""")


  }
}
