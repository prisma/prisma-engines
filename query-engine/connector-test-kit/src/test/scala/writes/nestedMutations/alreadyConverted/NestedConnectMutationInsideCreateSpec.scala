package writes.nestedMutations.alreadyConverted

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedConnectMutationInsideCreateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "a P1! to C1! relation with the child already in a relation" should "error when connecting by id since old required parent relation would be broken" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1 = t.child.where(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p1", p_1:"p", p_2: "1",
            |    childReq: {
            |      create: {c: "c1", c_1:"c", c_2: "1"}
            |    }
            |  }){
            |    childReq{
            |       ${t.child.selection}
            |    }
            |  }
            |}""",
            project
          ),
        "data.createParent.childReq"
      )

      server.queryThatMustFail(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2"
           |    childReq: {connect: $child1}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project,
        errorCode = 2014,
        errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models"""
      )

    }
  }

  "a P1! to C1 relation with the child already in a relation" should "should fail on existing old parent" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1 = t.child.where(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p1", p_1:"p", p_2: "1",
            |    childReq: {
            |      create: {c: "c1", c_1:"c", c_2: "1"}
            |    }
            |  }){
            |    childReq{
            |       ${t.child.selection}
            |    }
            |  }
            |}""",
            project
          ),
        "data.createParent.childReq"
      )

      server.queryThatMustFail(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2", p_1: "p", p_2: "2"
           |    childReq: {connect: $child1}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project,
        errorCode = 0, // 3042,
        errorContains = """InterpretationError(\"Error for binding \\'2\\': RelationViolation(RelationViolation { relation_name: \\\"ChildToParent\\\", model_a_name: \\\"Child\\\", model_b_name: \\\"Parent\\\" })"""
          // "The change you are trying to make would violate the required relation 'ChildToParent' between Child and Parent"
      )
    }
  }

  "a P1! to C1  relation with the child not in a relation" should "be connectable through a nested mutation by id" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)
      val looseChild = t.child.where(
        server
          .query(
            s"""mutation {
          |  createChild(data: {c: "looseChild", c_1:"c", c_2: "1"})
          |  {
          |    ${t.child.selection}
          |  }
          |}""",
            project
          ),
        "data.createChild"
      )

      val otherParentWithChildId = t.parent.where(
        server
          .query(
            s"""
           |mutation {
           |  createParent(data:{
           |    p: "otherParent", p_1:"p", p_2: "1",
           |    childReq: {create: {c: "otherChild", c_1:"c", c_2: "2"}}
           |  }){
           |     ${t.parent.selection}
           |  }
           |}
      """,
            project
          ),
        "data.createParent"
      )

      val child3Id = t.child.where(
        server
          .query(
            s"""mutation {
          |  createChild(data: {c: "c3", c_1:"c", c_2: "3"})
          |  {
          |     ${t.child.selection}
          |  }
          |}""",
            project
          ),
        "data.createChild"
      )

      val res = server.query(
        s"""
         |mutation {
         |  createParent(data:{
         |    p: "p2", p_1:"p", p_2: "2",
         |    childReq: {connect: $child3Id}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childReq":{"c":"c3"}}}}""")

      // verify preexisting data

      server
        .query(
          s"""
           |{
           |  parent(where: $otherParentWithChildId){
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
           |  child(where: $looseChild){
           |    c
           |  }
           |}
      """.stripMargin,
          project
        )
        .pathAsString("data.child.c") should be("looseChild")
    }
  }

  "a P1 to C1  relation with the child already in a relation" should "be connectable through a nested mutation by id if the child is already in a relation" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1 = t.child.where(
        server
          .query(
            s"""mutation {
            |  createParent(data: {
            |    p: "p1", p_1:"p", p_2: "1",
            |    childOpt: {
            |      create: {c: "c1", c_1:"c", c_2: "1"}
            |    }
            |  }){
            |    childOpt{
            |        ${t.child.selection}
            |    }
            |  }
            |}""",
            project
          ),
        "data.createParent.childOpt"
      )

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2", p_1:"p", p_2: "2",
           |    childOpt: {connect: $child1}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
        """,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

    }
  }

  "a P1 to C1  relation with the child without a relation" should "be connectable through a nested mutation by id" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1 = t.child.where(
        server
          .query(
            s"""mutation {
            |  createChild(data: {c: "c1", c_1:"c", c_2: "1"})
            |  {
            |     ${t.child.selection}
            |  }
            |}""",
            project
          ),
        "data.createChild"
      )

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p1", p_1:"p", p_2: "1",
           |    childOpt: {connect: $child1}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
        """,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

    }
  }

  "a PM to C1!  relation with the child already in a relation" should "be connectable through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child = t.child.whereFirst(
        server
          .query(
            s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1:"p", p_2: "1",
          |    childrenOpt: {
          |      create: {c: "c1", c_1:"c", c_2: "1"}
          |    }
          |  }){
          |    childrenOpt{
          |        ${t.child.selection}
          |    }
          |  }
          |}""",
            project
          ),
        "data.createParent.childrenOpt"
      )

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2", p_1:"p", p_2: "2",
           |    childrenOpt: {connect: $child}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"}]}}}""")
    }
  }

  "a P1 to C1!  relation with the child already in a relation" should "be connectable through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child = t.child.where(
        server
          .query(
            s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1:"p", p_2: "1",
          |    childOpt: {
          |      create: {c: "c1", c_1:"c", c_2: "1"}
          |    }
          |  }){
          |    childOpt{
          |        ${t.child.selection}
          |    }
          |  }
          |}""",
            project
          ),
        "data.createParent.childOpt"
      )

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2", p_1:"p", p_2: "2",
           |    childOpt: {connect: $child}
           |  }){
           |    childOpt {
           |      c
           |    }
           |  }
           |}
        """,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")
    }
  }

  "a PM to C1  relation with the child already in a relation" should "be connectable through a nested mutation by unique" in {
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
            |      create: [{c: "c1"}, {c: "c2"}]
            |    }
            |  }){
            |    childrenOpt{
            |       c
            |    }
            |  }
            |}""".stripMargin,
          project
        )

      // we are even resilient against multiple identical connects here -> twice connecting to c2

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2"
           |    childrenOpt: {connect: [{c: "c1"},{c: "c2"},{c: "c2"}]}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

    }
  }

  //todo

  "a PM to C1  relation with the child without a relation" should "be connectable through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Result = server
        .query(
          s"""mutation {
            |  createChild(data: {c: "c1", c_1: "c", c_2: "1"})
            |  {
            |    ${t.child.selection}
            |  }
            |}""".stripMargin,
          project
        )
      val child1Id = t.child.where(child1Result, "data.createChild")

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2"
           |    childrenOpt: {connect: $child1Id}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"}]}}}""")

    }
  }

  "a PM to C1 relation with a child without a relation" should "error if also trying to connect to a non-existing node" in { // TODO: Remove when transactions are back
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Result = server
        .query(
          s"""mutation {
            |  createChild(data: {c: "c1"})
            |  {
            |    ${t.child.selection}
            |  }
            |}""".stripMargin,
          project
        )
      val child1Id = t.child.where(child1Result, "data.createChild")

      server.queryThatMustFail(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2"
           |    childrenOpt: {connect: [$child1Id, {c: "DOES NOT EXIST"}]}
           |  }){
           |    childrenOpt {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project,
        errorCode = 0, // 3039,
        errorContains = "RecordNotFound(\\\"Expected 2 records to be connected, found 1.\\\")"
      )

    }
  }

  "a P1! to CM  relation with the child already in a relation" should "be connectable through a nested mutation by unique" in {
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
        |      create: {
        |        c: "c1"
        |        c_1: "c_1"
        |        c_2: "c_2"
        |      }
        |    }
        |  }){
        |    childReq{
        |       c
        |    }
        |  }
        |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
         |mutation {
         |  createParent(data:{
         |    p: "p2"
         |    childReq: {connect: {c: "c1"}}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childReq":{"c":"c1"}}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}""")

    }
  }

  "a P1! to CM  relation with the child not already in a relation" should "be connectable through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
          |  createChild(data: {
          |    c: "c1"
          |    c_1: "c_1"
          |    c_2: "c_2"
          |  }){
          |       c
          |  }
          |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
           |mutation {
           |  createParent(data:{
           |    p: "p2"
           |    childReq: {connect: {c: "c1"}}
           |  }){
           |    childReq {
           |      c
           |    }
           |  }
           |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childReq":{"c":"c1"}}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p2"}]}]}}""")

    }
  }

  "a P1 to CM  relation with the child already in a relation" should "be connectable through a nested mutation by unique" in {
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
        |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
         |mutation {
         |  createParent(data:{
         |    p: "p2"
         |    childOpt: {connect: {c: "c1"}}
         |  }){
         |    childOpt{
         |      c
         |    }
         |  }
         |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}""")

    }
  }

  "a P1 to CM  relation with the child not already in a relation" should "be connectable through a nested mutation by unique" in {
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
        |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
         |mutation {
         |  createParent(data:{
         |    p: "p2"
         |    childOpt: {connect: {c: "c1"}}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p2"}]}]}}""")

    }
  }

  "a PM to CM  relation with the children already in a relation" should "be connectable through a nested mutation by unique" in {
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
        |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
         |mutation {
         |  createParent(data:{
         |    p: "p2"
         |    childrenOpt: {connect: [{c: "c1"}, {c: "c2"}]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"parentsOpt":[{"p":"p1"},{"p":"p2"}]}]}}""")

    }
  }

  "a PM to CM  relation with the child not already in a relation" should "be connectable through a nested mutation by unique" in {
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
        |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
         |mutation {
         |  createParent(data:{
         |    p: "p2"
         |    childrenOpt: {connect: {c: "c1"}}
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"}]}}}""")

      server.query(s"""query{children{parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p2"}]}]}}""")

    }
  }

  // todo check whether this is already covered

  "a PM to CM  relation without a backrelation" should "be connectable through a nested mutation by unique" ignore {

    val testDataModels = {
      val s1 =
        """model Role {
          | id String @id @default(cuid())
          | r  String @unique
          |}
          |
          |model User {
          | id    String @id @default(cuid())
          | u     String @unique
          | roles Role[] @relation(references: [id])
          |}
        """

      val s2 =
        """model Role {
          | id String @id @default(cuid())
          | r  String @unique
          |}
          |
          |model User {
          | id    String @id @default(cuid())
          | u     String @unique
          | roles Role[]
          |}
        """
      TestDataModels(mongo = Vector(s1), sql = Vector(s2))
    }

    testDataModels.testV11 { project =>
      server.query(
        """mutation {
          |  createRole(data: {r: "r1"}){
          |       r
          |  }
          |}""".stripMargin,
        project
      )

      val res = server.query(
        s"""
           |mutation {
           |  createUser(data:{
           |    u: "u2"
           |    roles: {connect: {r: "r1"}}
           |  }){
           |    roles {
           |      r
           |    }
           |  }
           |}
      """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createUser":{"roles":[{"r":"r1"}]}}}""")

    }
  }

  "A PM to C1 relation" should "throw a proper error if connected by wrong id" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo {
        | id       String    @id @default(cuid())
        | comments Comment[]
        |}
        |
        |model Comment {
        | id    String  @id @default(cuid())
        | text  String
        | todo  Todo?   @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    server.queryThatMustFail(
      s"""
         |mutation {
         |  createTodo(data:{
         |    comments: {
         |      connect: [{id: "5beea4aa6183dd734b2dbd9b"}]
         |    }
         |  }){
         |    id
         |    comments {
         |      id
         |      text
         |    }
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 0, // 3039,
      errorContains = "RecordNotFound(\\\"Expected 1 records to be connected, found 0.\\\")"
    )
  }

  "A P1 to CM relation " should "throw a proper error if connected by wrong id the other way around" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment {
        | id   String @id @default(cuid())
        | text String
        | todo Todo?  @relation(references: [id])
        |}
        |
        |model Todo {
        | id       String   @id @default(cuid())
        | text     String?
        | comments Comment[]
        |}
      """.stripMargin
    }
    database.setup(project)

    server.queryThatMustFail(
      s"""
         |mutation {
         |  createComment(data:{
         |    text: "bla"
         |    todo: {
         |      connect: {id: "5beea4aa6183dd734b2dbd9b"}
         |    }
         |  }){
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 0, // 3039,
      errorContains = """Error occurred during query execution:\nInterpretationError(\"Error for binding \\'1\\': AssertionError(\\\"[Query Graph] Expected a valid parent ID to be present for a nested connect on a one-to-many relation.\\\")\"""
    )
  }

  "A PM to C1 relation" should "throw a proper error if the id of a wrong model is provided" ignore {
    val project = SchemaDsl.fromStringV11() {
      """model Todo {
        | id        String    @id @default(cuid())
        | comments  Comment[]
        |}
        |
        |model Comment {
        | id   String @id @default(cuid())
        | text String
        | todo Todo?  @relation(references: [id])
        |}
      """.stripMargin
    }
    database.setup(project)

    val comment1Id = server.query("""mutation { createComment(data: {text: "comment1"}){ id } }""", project).pathAsString("data.createComment.id")
    val comment2Id = server.query("""mutation { createComment(data: {text: "comment2"}){ id } }""", project).pathAsString("data.createComment.id")

    val todoId = server
      .query(
        s"""
           |mutation {
           |  createTodo(data:{
           |    comments: {
           |      connect: [{id: "$comment1Id"}, {id: "$comment2Id"}]
           |    }
           |  }){
           |    id
           |  }
           |}
      """.stripMargin,
        project
      )
      .pathAsString("data.createTodo.id")

    server.queryThatMustFail(
      s"""
         |mutation {
         |  createTodo(data:{
         |    comments: {
         |      connect: [{id: "$todoId"}]
         |    }
         |  }){
         |    id
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 0, // 3039,
      errorContains = """RecordNotFound(\"Expected 1 records to be connected, found 0.\")"""
    )
  }
}
