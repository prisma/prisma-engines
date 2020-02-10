package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedDeleteManyMutationInsideUpdateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "A 1-n relation" should "error if trying to use nestedDeleteMany" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parent1Id = server
        .query(
          """mutation {
          |  createParent(data: {p: "p1"})
          |  {
          |    id
          |  }
          |}""",
          project
        )
        .pathAsString("data.createParent.id")

      val res = server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where:{id: "$parent1Id"}
         |  data:{
         |    p: "p2"
         |    childOpt: {deleteMany: {
         |        where:{c: "c"}
         |    }}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 0,
        errorContains = """ Reason: 'childOpt.deleteMany' Field 'deleteMany' is not defined in the input model 'ChildUpdateOneWithoutParentOptInput'."""
      )
    }
  }

  "a PM to C1!  relation " should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      setupData(project)

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: {p: "p1"}
         |    data:{
         |    childrenOpt: {deleteMany: {c_contains:"c"}
         |    }
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      server.query("query{parents{p,childrenOpt{c}}}", project).toString() should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}""")
    }
  }

  "a PM to C1  relation " should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      setupData(project)

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: {p: "p1"}
         |    data:{
         |    childrenOpt: {deleteMany: {c_contains:"c"}
         |   }
         |  }){
         |    childrenOpt {
         |      c
         |
         |    }
         |  }
         |}
      """,
        project
      )

      server.query("query{parents{p,childrenOpt{c}}}", project).toString() should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}""")
    }
  }

  "a PM to CM  relation " should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      setupData(project)

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: {p: "p1"}
         |    data:{
         |    childrenOpt: {deleteMany: {
         |          c_contains:"c"
         |      }
         |    }
         |  }){
         |    childrenOpt {
         |      c
         |
         |    }
         |  }
         |}
      """,
        project
      )

      server.query("query{parents{p,childrenOpt{c}}}", project).toString() should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}""")
    }
  }

  "a PM to C1!  relation " should "work with several deleteManys" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      setupData(project)

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: {p: "p1"}
         |    data:{
         |    childrenOpt: {deleteMany: [
         |    {
         |        c_contains:"1"
         |    },
         |    {
         |        c_contains:"2"
         |    }
         |    ]}
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      server.query("query{parents{p,childrenOpt{c}}}", project).toString() should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}""")

    }
  }

  "a PM to C1!  relation " should "work with empty Filter" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      setupData(project)

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: {p: "p1"}
         |    data:{
         |    childrenOpt: {deleteMany: [
         |    {}
         |    ]}
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      server.query("query{parents{p,childrenOpt{c}}}", project).toString() should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}""")
    }
  }

  "a PM to C1!  relation " should "not change anything when there is no hit" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      setupData(project)

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: {p: "p1"}
         |    data:{
         |    childrenOpt: {deleteMany: [
         |    {
         |        c_contains:"3"
         |    },
         |    {
         |        c_contains:"4"
         |    }
         |    ]}
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      server.query("query{parents{p,childrenOpt{c}}}", project).toString() should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[{"c":"c1"},{"c":"c2"}]},{"p":"p2","childrenOpt":[{"c":"c3"},{"c":"c4"}]}]}}""")
    }
  }

  private def setupData(project: Project) = {
    server.query(
      """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    childrenOpt: {
        |      create: [{c: "c1"},{c: "c2"}]
        |    }
        |  }){
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""",
      project
    )

    server.query(
      """mutation {
        |  createParent(data: {
        |    p: "p2"
        |    childrenOpt: {
        |      create: [{c: "c3"},{c: "c4"}]
        |    }
        |  }){
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""",
      project
    )
  }

}
