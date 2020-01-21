package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedConnectMutationInsideUpdateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "A P1! to C1! relation with the child already in a relation" should "error when connecting since old required parent relation would be broken" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val childIdentifier = t.child.parse(
        server
          .query(
            s"""mutation {
             |  createParent(data: {
             |    p: "p1", p_1: "p", p_2: "1",
             |    childReq: {
             |      create: {c: "c1", c_1: "c", c_2: "1"}
             |    }
             |  }){
             |    childReq{
             |       ${t.child.returnValue}
             |    }
             |  }
             |}""",
            project
          ),
        "data.createParent.childReq"
      )

      val parentIdentifier = t.parent.parse(
        server
          .query(
            s"""mutation {
             |  createParent(data: {
             |    p: "p2", p_1: "p", p_2: "2",
             |    childReq: {
             |      create: {c: "c2", c_1: "c", c_2: "2"}
             |    }
             |  }){
             |  ${t.parent.returnValue}
             |  }
             |}""",
            project
          ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      server.queryThatMustFail(
        s"""mutation {
           |  updateParent(
           |  where: {${t.parent.identifierName}: "$parentIdentifier"}
           |  data:{
           |    childReq: {connect: { ${t.child.identifierName}: "$childIdentifier"}}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}""",
        project,
        errorCode = 3042,
        errorContains = "The change you are trying to make would violate the required relation 'ChildToParent' between Child and Parent"
      )

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

    }
  }

  "a P1! to C1 relation with the child already in a relation" should "should fail on existing old parent" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Id = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p1", p_1: "p", p_2: "1",
            |    childReq: {
            |      create: {c: "c1", c_1: "c", c_2: "1"}
            |    }
            |  }){
            |    childReq{
            |       ${t.child.returnValue}
            |    }
            |  }
            |}""",
            project
          ),
        "data.createParent.childReq"
      )

      val parentId2 = t.parent.parse(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p2", p_1: "p", p_2: "2",
            |    childReq: {
            |      create: {c: "c2"}
            |    }
            |  }){
            |    ${t.parent.returnValue}
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      server.queryThatMustFail(
        s"""mutation {
           |  updateParent(
           |  where: {${t.parent.identifierName}: "$parentId2"}
           |  data:{
           |    childReq: {connect: {${t.child.identifierName}: "$child1Id"}}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
      """,
        project,
        errorCode = 3042,
        errorContains = "The change you are trying to make would violate the required relation 'ChildToParent' between Child and Parent"
      )

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

    }
  }

  "a P1! to C1  relation with the child not in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val looseChildId = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createChild(data: {c: "looseChild"})
            |  {
            |    ${t.child.returnValue}
            |  }
            |}""".stripMargin,
            project
          ),
        "data.createChild"
      )

      val otherParentWithChildId = t.parent.parse(
        server
          .query(
            s"""
             |mutation {
             |  createParent(data:{
             |    p: "otherParent"
             |    childReq: {create: {c: "otherChild"}}
             |  }){
             |    ${t.parent.returnValue}
             |  }
             |}
      """.stripMargin,
            project
          ),
        "data.createParent"
      )

      val child1Id = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createChild(data: {c: "c1"})
            |  {
            |    ${t.child.returnValue}
            |  }
            |}""",
            project
          ),
        "data.createChild"
      )

      val parentId = t.parent.parse(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p2"
            |    childReq: {
            |      create: {c: "c2"}
            |    }
            |  }){
            |     ${t.parent.returnValue}
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {${t.parent.identifierName}: "$parentId"}
           |  data:{
           |    p: "p2"
           |    childReq: {connect: {${t.child.identifierName}: "$child1Id"}}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childReq":{"c":"c1"}}}}""")
      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      // verify preexisting data

      server
        .query(
          s"""
             |{
             |  parent(where: {${t.parent.identifierName}: "${otherParentWithChildId}"}){
             |    childReq {
             |      c
             |    }
             |  }
             |}
      """.stripMargin,
          project
        )
        .pathAsString("data.parent.childReq.c") should be("otherChild")

      server
        .query(
          s"""
             |{
             |  child(where: {${t.child.identifierName}: "${looseChildId}"}){
             |    c
             |  }
             |}
      """.stripMargin,
          project
        )
        .pathAsString("data.child.c") should be("looseChild")
    }
  }

  "a P1 to C1  relation with the child already in a relation" should "be connectable through a nested mutation if the child is already in a relation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val looseChildId = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createChild(data: {c: "looseChild"})
            |  {
            |    ${t.child.returnValue}
            |  }
            |}""".stripMargin,
            project
          ),
        "data.createChild"
      )

      val otherParentWithChildId = t.parent.parse(
        server
          .query(
            s"""
             |mutation {
             |  createParent(data:{
             |    p: "otherParent"
             |    childOpt: {create: {c: "otherChild"}}
             |  }){
             |    ${t.parent.returnValue}
             |  }
             |}
      """.stripMargin,
            project
          ),
        "data.createParent"
      )

      val child1Id = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p1"
            |    childOpt: {
            |      create: {c: "c1"}
            |    }
            |  }){
            |    childOpt{
            |       ${t.child.returnValue}
            |    }
            |  }
            |}""",
            project
          ),
        "data.createParent.childOpt"
      )

      val parentId2 = t.parent.parse(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p2"
            |    childOpt: {
            |      create: {c: "c2"}
            |    }
            |  }){
            |    ${t.parent.returnValue}
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(3) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where:{${t.parent.identifierName}: "$parentId2"}
           |  data:{
           |    p: "p2"
           |    childOpt: {connect: {${t.child.identifierName}: "$child1Id"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      // verify preexisting data

      server
        .query(
          s"""
             |{
             |  parent(where: {${t.parent.identifierName}: "${otherParentWithChildId}"}){
             |    childOpt {
             |      c
             |    }
             |  }
             |}
      """.stripMargin,
          project
        )
        .pathAsString("data.parent.childOpt.c") should be("otherChild")

      server
        .query(
          s"""
             |{
             |  child(where: {${t.child.identifierName}: "${looseChildId}"}){
             |    c
             |  }
             |}
      """.stripMargin,
          project
        )
        .pathAsString("data.child.c") should be("looseChild")
    }
  }

  "a P1 to C1  relation with the child and the parent without a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Id = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createChild(data: {c: "c1"})
            |  {
            |    ${t.child.returnValue}
            |  }
            |}""",
            project
          ),
        "data.createChild"
      )

      val parent1Id = t.child.parse(
        server
          .query(
            """mutation {
            |  createParent(data: {p: "p1"})
            |  {
            |    id
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(0) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where:{id: "$parent1Id"}
           |  data:{
           |    p: "p2"
           |    childOpt: {connect: {id: "$child1Id"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "a P1 to C1  relation with the child without a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Id = t.child.parse(
        server
          .query(
            s"""mutation {
            |  createChild(data: {c: "c1"})
            |  {
            |    ${t.child.returnValue}
            |  }
            |}""",
            project
          ),
        "data.createChild"
      )

      val parentId = t.parent.parse(
        server
          .query(
            """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    childOpt: {
            |      create: {c: "c2"}
            |    }
            |  }){
            |    id
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where:{id: "$parentId"}
           |  data:{
           |    p: "p2"
           |    childOpt: {connect: {id: "$child1Id"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "a P1 to C1  relation with the parent without a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentId = t.parent.parse(
        server
          .query(
            """mutation {
            |  createParent(data: {p: "p1"})
            |  {
            |    id
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      val childId = t.child.parse(
        server
          .query(
            """mutation {
            |  createParent(data: {
            |    p: "p2"
            |    childOpt: {
            |      create: {c: "c1"}
            |    }
            |  }){
            |    childOpt{id}
            |  }
            |}""",
            project
          ),
        "data.createParent.childOpt"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where:{id: "$parentId"}
           |  data:{
           |    childOpt: {connect: {id: "$childId"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "A PM to CM relation connecting two nodes twice" should "not error" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentId = t.child.parse(
        server
          .query(
            """mutation {
            |  createParent(data: {p: "p1"})
            |  {
            |    id
            |  }
            |}""",
            project
          ),
        "data.createParent"
      )

      val childId = t.child.parse(
        server
          .query(
            """mutation {
            |  createParent(data: {
            |    p: "p2"
            |    childrenOpt: {
            |      create: {c: "c1"}
            |    }
            |  }){
            |    childrenOpt{id}
            |  }
            |}""",
            project
          ),
        "data.createParent.childrenOpt"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where:{id: "$parentId"}
           |  data:{
           |    childrenOpt: {connect: {id: "$childId"}}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"}]}}}""")

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      val res2 = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where:{id: "$parentId"}
           |  data:{
           |    childrenOpt: {connect: {id: "$childId"}}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res2 should be(res)

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      server.query("""query{parents{p, childrenOpt{c}}}""", project).toString should be(
        """{"data":{"parents":[{"p":"p1","childrenOpt":[{"c":"c1"}]},{"p":"p2","childrenOpt":[{"c":"c1"}]}]}}""")
    }

  }

  "a PM to C1!  relation with the child already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val otherParentWithChildId = t.parent.parse(
        server
          .query(
            s"""
             |mutation {
             |  createParent(data:{
             |    p: "otherParent"
             |    childrenOpt: {create: {c: "otherChild"}}
             |  }){
             |     ${t.parent.returnValue}
             |  }
             |}
      """.stripMargin,
            project
          ),
        "data.createParent"
      )

      val childId = t.child.parseFirst(
        server
          .query(
            s"""mutation {
          |  createParent(data: {
          |    p: "p1"
          |    childrenOpt: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    childrenOpt{
          |        ${t.child.returnValue}
          |    }
          |  }
          |}""",
            project
          ),
        "data.createParent.childrenOpt"
      )

      server.query(
        s"""mutation {
          |  createParent(data: {
          |    p: "p2"
          |    childrenOpt: {
          |      create: {c: "c2"}
          |    }
          |  }){
          |    childrenOpt{
          |        ${t.child.returnValue}
          |    }
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(3) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |    where: {p: "p2"}
           |    data:{
           |    childrenOpt: {connect: {${t.child.identifierName}: "$childId"}}
           |  }){
           |    childrenOpt(first:10) {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(3) }

      // verify preexisting data

      server
        .query(
          s"""
             |{
             |  parent(where: { ${t.parent.identifierName}: "${otherParentWithChildId}"}){
             |    childrenOpt {
             |      c
             |    }
             |  }
             |}
      """.stripMargin,
          project
        )
        .pathAsString("data.parent.childrenOpt.[0].c") should be("otherChild")
    }
  }

  "a P1 to C1!  relation with the child and the parent already in a relation" should "should error in a nested mutation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createParent(data: {
          |    p: "p1"
          |    childOpt: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    childOpt{
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
          |    childOpt: {
          |      create: {c: "c2"}
          |    }
          |  }){
          |    childOpt{
          |       c
          |    }
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      server.queryThatMustFail(
        s"""mutation {
           |  updateParent(
           |  where: {p: "p2"}
           |  data:{
           |    childOpt: {connect: {c: "c1"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project,
        errorCode = 3042,
        errorContains = "The change you are trying to make would violate the required relation 'ChildToParent' between Child and Parent"
      )

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }
    }
  }
  //todo unique tests
// these need to explicitly fetch the return values

  "a P1 to C1! relation with the child already in a relation" should "should not error when switching to a different parent" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child = t.child.parse(
        server.query(
          """mutation {
          |  createParent(data: {
          |    p: "p1"
          |    childOpt: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    childOpt{
          |       c
          |    }
          |  }
          |}""",
          project
        ),
        "data.createParent.childOpt"
      )

      val parent = t.parent.parse(
        server.query(
          """mutation {
          |  createParent(data: {
          |    p: "p2"
          |  }){
          |  p
          |  }
          |}""",
          project
        ),
        "data.createParent"
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {p: "p2"}
           |  data:{
           |    childOpt: {connect: {${t.child.identifierName}: "$child"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "a PM to C1  relation with the child already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    childrenOpt: {
            |      create: [{c: "c1"}, {c: "c2"}, {c: "c3"}]
            |    }
            |  }){
            |    childrenOpt{
            |       c
            |    }
            |  }
            |}""",
          project
        )

      server
        .query(
          """mutation {
            |  createParent(data: {p: "p2"}){
            |    p
            |  }
            |}""",
          project
        )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      // we are even resilient against multiple identical connects here -> twice connecting to c2

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: { p: "p2"}
           |  data:{
           |    childrenOpt: {connect: [{c: "c1"},{c: "c2"},{c: "c2"}]}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      server.query("""query{parent(where:{p: "p1"}){childrenOpt{c}}}""", project).toString should be("""{"data":{"parent":{"childrenOpt":[{"c":"c3"}]}}}""")
    }
  }

  "a PM to C1  relation with the child without a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server
        .query(
          """mutation {
            |  createChild(data: {c: "c1"})
            |  {
            |    id
            |  }
            |}""",
          project
        )

      server
        .query(
          """mutation {
            |  createParent(data: {p: "p1"})
            |  {
            |    id
            |  }
            |}""",
          project
        )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(0) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {p:"p1"}
           |  data:{
           |    childrenOpt: {connect: {c: "c1"}}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"}]}}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "a P1! to CM  relation with the child already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createParent(data: {
          |    p: "p1"
          |    childReq: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    childReq{
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
          |    childReq: {
          |      create: {c: "c2"}
          |    }
          |  }){
          |    childReq{
          |       c
          |    }
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {p: "p2"}
           |  data:{
           |    childReq: {connect: {c: "c1"}}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childReq":{"c":"c1"}}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c2","parentsOpt":[]}]}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }
    }
  }

  "a P1! to CM  relation with the child not already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createChild(data: {c: "c1"}){
          |       c
          |  }
          |}""",
        project
      )

      server.query(
        """mutation {
          |  createParent(data: {
          |    p: "p2"
          |    childReq: {
          |      create: {c: "c2"}
          |    }
          |  }){
          |    childReq{
          |       c
          |    }
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {p: "p2"}
           |  data:{
           |    childReq: {connect: {c: "c1"}}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childReq":{"c":"c1"}}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p2"}]},{"c":"c2","parentsOpt":[]}]}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "a P1 to CM  relation with the child already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createParent(data: {
          |    p: "p1"
          |    childOpt: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    childOpt{
          |       c
          |    }
          |  }
          |}""",
        project
      )

      server.query(
        """mutation {
          |  createParent(data: {p: "p2"}){
          |    p
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |    where: {p: "p2"}
           |    data:{
           |    childOpt: {connect: {c: "c1"}}
           |  }){
           |    childOpt{
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(2) }
    }
  }

  "a P1 to CM  relation with the child not already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createChild(data: {c: "c1"}){
          |       c
          |  }
          |}""",
        project
      )

      server.query(
        """mutation {
          |  createParent(data: {p: "p1"}){
          |       p
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(0) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {p: "p1"}
           |  data:{
           |    childOpt: {connect: {c: "c1"}}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c1"}}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]}]}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  "a PM to CM  relation with the children already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

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

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(4) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {    p: "p2"}
           |  data:{
           |    childrenOpt: {connect: [{c: "c1"}, {c: "c2"}]}
           |  }){
           |    childrenOpt(orderBy: id_ASC){
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c3"},{"c":"c4"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c2","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c3","parentsOpt":[{"p":"p2"}]},{"c":"c4","parentsOpt":[{"p":"p2"}]}]}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(6) }
    }
  }

  "a PM to CM  relation with the child not already in a relation" should "be connectable through a nested mutation" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createChild(data: {c: "c1"}){
          |       c
          |  }
          |}""",
        project
      )

      server.query(
        """mutation {
          |  createParent(data: {p: "p1"}){
          |       p
          |  }
          |}""",
        project
      )

      // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(0) }

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: {p: "p1"}
           |  data:{
           |    childrenOpt: {connect: {c: "c1"}}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"}]}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p1"}]}]}}""")

    // ifConnectorIsActive { dataResolver(project).countByTable("_ChildToParent").await should be(1) }
    }
  }

  //todo other tests check whether already covered

  "MARCUS A PM to C1 relation" should "be connectable by id through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo {
        | id       String    @id @default(cuid())
        | comments Comment[]
        |}
        |
        |model Comment {
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo?   @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    val todoId     = server.query("""mutation { createTodo(data: {}){ id } }""", project).pathAsString("data.createTodo.id")
    val comment1Id = server.query("""mutation { createComment(data: {text: "comment1"}){ id } }""", project).pathAsString("data.createComment.id")
    val comment2Id = server.query("""mutation { createComment(data: {text: "comment2"}){ id } }""", project).pathAsString("data.createComment.id")

    val result = server.query(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        connect: [{id: "$comment1Id"}, {id: "$comment2Id"}]
         |      }
         |    }
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateTodo.comments").toString, """[{"text":"comment1"},{"text":"comment2"}]""")
  }

  "MARCUS A PM to C1 relation" should "be connectable by any unique argument through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo {
        | id       String    @id @default(cuid())
        | comments Comment[]
        |}
        |
        |model Comment {
        | id    String  @id @default(cuid())
        | text  String?
        | alias String  @unique
        | todo  Todo?   @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    val todoId = server.query("""mutation { createTodo(data: {}){ id } }""", project).pathAsString("data.createTodo.id")
    server.query("""mutation { createComment(data: {text: "comment1", alias: "alias1"}){ id } }""", project).pathAsString("data.createComment.id")
    server.query("""mutation { createComment(data: {text: "comment2", alias: "alias2"}){ id } }""", project).pathAsString("data.createComment.id")

    val result = server.query(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        connect: [{alias: "alias1"}, {alias: "alias2"}]
         |      }
         |    }
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateTodo.comments").toString, """[{"text":"comment1"},{"text":"comment2"}]""")
  }

  "MARCUS A P1 to CM relation" should "be connectable by id through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment {
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo?   @relation(references: [id])
        |}
        |
        |model Todo {
        | id       String @id @default(cuid())
        | title    String
        | comments Comment[]
        |}
      """.stripMargin
    }
    database.setup(project)

    val commentId = server.query("""mutation { createComment(data: {}){ id } }""", project).pathAsString("data.createComment.id")
    val todoId    = server.query("""mutation { createTodo(data: { title: "the title" }){ id } }""", project).pathAsString("data.createTodo.id")

    val result = server.query(
      s"""
         |mutation {
         |  updateComment(
         |    where: {
         |      id: "$commentId"
         |    }
         |    data: {
         |      todo: {
         |        connect: {id: "$todoId"}
         |      }
         |    }
         |  ){
         |    id
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsString("data.updateComment.todo.title"), "the title")
  }

  "MARCUS A P1 to C1 relation" should "be connectable by id through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """model Note {
        | id    String  @id @default(cuid())
        | text  String?
        | todo  Todo?   @relation(references: [id])
        |}
        |
        |model Todo {
        | id     String @id @default(cuid())
        | title  String
        | note   Note?
        |}
      """.stripMargin
    }
    database.setup(project)

    val noteId = server.query("""mutation { createNote(data: {}){ id } }""", project).pathAsString("data.createNote.id")
    val todoId = server.query("""mutation { createTodo(data: { title: "the title" }){ id } }""", project).pathAsString("data.createTodo.id")

    val result = server.query(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    data: {
         |      todo: {
         |        connect: {id: "$todoId"}
         |      }
         |    }
         |  ){
         |    id
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsString("data.updateNote.todo.title"), "the title")
  }

  "MARCUS A P1 to C1 relation" should "connecting nodes by id through a nested mutation should not error when items are already connected" in {
    val project = SchemaDsl.fromStringV11() {
      """model Note {
        | id    String @id @default(cuid())
        | text  String?
        | todo  Todo?
        |}
        |
        |model Todo {
        | id     String  @id @default(cuid())
        | title  String
        | note   Note?   @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    val noteId = server.query("""mutation { createNote(data: {}){ id } }""", project).pathAsString("data.createNote.id")
    val todoId = server.query("""mutation { createTodo(data: { title: "the title" }){ id } }""", project).pathAsString("data.createTodo.id")

    val result = server.query(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    data: {
         |      todo: {
         |        connect: {id: "$todoId"}
         |      }
         |    }
         |  ){
         |    id
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsString("data.updateNote.todo.title"), "the title")

    server.query(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    data: {
         |      todo: {
         |        connect: {id: "$todoId"}
         |      }
         |    }
         |  ){
         |    id
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )
  }

  "MARCUS A PM to C1 relation" should "be connectable by any unique argument through a nested mutation 2" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo {
        | id       String    @id @default(cuid())
        | comments Comment[]
        |}
        |
        |model Comment {
        | id    String  @id @default(cuid())
        | text  String?
        | alias String  @unique
        | todo  Todo?   @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    val todoId = server.query("""mutation { createTodo(data: {}){ id } }""", project).pathAsString("data.createTodo.id")
    server.query("""mutation { createComment(data: {text: "comment1", alias: "alias1"}){ id } }""", project).pathAsString("data.createComment.id")
    server.query("""mutation { createComment(data: {text: "comment2", alias: "alias2"}){ id } }""", project).pathAsString("data.createComment.id")

    val result = server.query(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        connect: [{alias: "alias1"}, {alias: "alias2"}]
         |      }
         |    }
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateTodo.comments").toString, """[{"text":"comment1"},{"text":"comment2"}]""")
  }

  "MARCUS A PM to C1 relation" should "be connectable through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo {
        | id       String    @id @default(cuid())
        | title    String?   @unique
        | comments Comment[]
        |}
        |
        |model Comment {
        | id   String  @id @default(cuid())
        | text String? @unique
        | todo Todo?   @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query("""mutation { createTodo(data: {title: "todo"}){ id } }""", project).pathAsString("data.createTodo.id")
    server.query("""mutation { createComment(data: {text: "comment1"}){ id } }""", project).pathAsString("data.createComment.id")
    server.query("""mutation { createComment(data: {text: "comment2"}){ id } }""", project).pathAsString("data.createComment.id")

    val result = server.query(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      title: "todo"
         |    }
         |    data:{
         |      comments: {
         |        connect: [{text: "comment1"}, {text: "comment2"}]
         |      }
         |    }
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateTodo.comments").toString, """[{"text":"comment1"},{"text":"comment2"}]""")
  }

  "MARCUS a PM to CM  self relation with the child not already in a relation" should "be connectable through a nested mutation" in {
    val testDataModels = {
      val s1 =
        """model Technology {
          | id                 String       @id @default(cuid())
          | name               String       @unique
          | childTechnologies  Technology[] @relation(name: "ChildTechnologies", references: [id])
          | parentTechnologies Technology[] @relation(name: "ChildTechnologies")
          |}
        """

      val s2 =
        """model Technology {
          | id                 String       @id @default(cuid())
          | name               String       @unique
          | childTechnologies  Technology[] @relation(name: "ChildTechnologies")
          | parentTechnologies Technology[] @relation(name: "ChildTechnologies", references: [id])
          |}
        """

      val s3 =
        """model Technology {
          | id                 String       @id @default(cuid())
          | name               String       @unique
          | childTechnologies  Technology[] @relation(name: "ChildTechnologies")
          | parentTechnologies Technology[] @relation(name: "ChildTechnologies")
          |}
        """
      TestDataModels(mongo = Vector(s1, s2), sql = Vector(s3))
    }

    testDataModels.testV11 { project =>
      server.query("""mutation {createTechnology(data: {name: "techA"}){name}}""", project)

      server.query("""mutation {createTechnology(data: {name: "techB"}){name}}""", project)

      val res = server.query(
        s"""mutation {
           |  updateTechnology(where: {name: "techA"},
           |                   data:  {childTechnologies: {connect: {name: "techB"}}})
           |      {name,
           |       childTechnologies  {name}
           |       parentTechnologies {name}}
           |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateTechnology":{"name":"techA","childTechnologies":[{"name":"techB"}],"parentTechnologies":[]}}}""")

      val res2 = server.query(
        s"""query {
           |  technologies{
           |       name
           |       childTechnologies  {name}
           |       parentTechnologies {name}
           |  }
           |}
      """,
        project
      )

      res2.toString should be(
        """{"data":{"technologies":[{"name":"techA","childTechnologies":[{"name":"techB"}],"parentTechnologies":[]},{"name":"techB","childTechnologies":[],"parentTechnologies":[{"name":"techA"}]}]}}""")
    }
  }
}
