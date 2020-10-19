package writes.nestedMutations.alreadyConverted

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedDeleteMutationInsideUpdateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "a P1! to C1! relation " should "error when deleting the child" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1",
          |    childReq: {
          |      create: {c: "c1", c_1: "c", c_2: "1"}
          |    }
          |  }){
          |    ${t.parent.selection}
          |  }
          |}""",
          project
        )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    p: { set: "p2" }, p_1: { set: "p" }, p_2: { set: "2" },
         |    childReq: {delete: true}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2009,
        errorContains =
          "`Field does not exist on enclosing type.` at `Mutation.updateParent.data.ParentUpdateInput.childReq.ChildUpdateOneRequiredWithoutParentReqInput.delete`"
      )
    }
  }

  "a P1! to C1 relation" should "always fail when trying to delete the child" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1",
          |    childReq: {
          |      create: {c: "c1", c_1: "c", c_2: "1"}
          |    }
          |  }){
          |  ${t.parent.selection}
          |  }
          |}""",
          project
        )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    p: { set: "p2" }
         |    childReq: {delete: true}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2009,
        errorContains =
          "`Field does not exist on enclosing type.` at `Mutation.updateParent.data.ParentUpdateInput.childReq.ChildUpdateOneRequiredWithoutParentOptInput.delete`"
      )
    }
  }

  "a P1 to C1  relation " should "work through a nested mutation by id" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val existingDataRes = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "existingParent", p_1: "p", p_2: "1",
          |    childOpt: {
          |      create: {c: "existingChild", c_1: "c", c_2: "1"}
          |    }
          |  }){
          |    ${t.parent.selection}
          |  }
          |}""",
          project
        )

      val parentIdentifier = t.parent.where(existingDataRes, "data.createParent")

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p2", p_1: "p", p_2: "2",
          |    childOpt: {
          |      create: {c: "c2",, c_1: "c", c_2: "2"}
          |    }
          |  }){
          |    ${t.parent.selection}
          |  }
          |}""",
          project
        )

      val parentIdentifier2 = t.parent.where(res, "data.createParent")

      val res2 = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier2
         |  data:{
         |    childOpt: {delete: true}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childOpt":null}}}""")

      // Verify existing data

      server
        .query(
          s"""
         |{
         |  parent(where: $parentIdentifier ){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
          project
        )
        .toString should be(s"""{"data":{"parent":{"childOpt":{"c":"existingChild"}}}}""")
    }
  }

  "a P1 to C1  relation" should "error if the nodes are not connected" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server
        .query(
          s"""mutation {
          |  createChild(data: {c: "c1", c_1: "c", c_2: "1"})
          |  {
          |    ${t.child.selection}
          |  }
          |}""",
          project
        )

      val parentIdentifier = t.parent.where(
        server
          .query(
            s"""mutation {
          |  createParent(data: {p: "p1", p_1: "p", p_2: "1",})
          |  {
          |    ${t.parent.selection}
          |  }
          |}""",
            project
          ),
        "data.createParent"
      )

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where:$parentIdentifier
         |  data:{
         |    childOpt: {delete: true}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2016,
        errorContains = """[Query Graph] Expected a valid parent ID to be present for a nested delete on a one-to-many relation."""
      )
    }
  }

  "a PM to C1!  relation " should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)
      val res = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1",
        |    childrenOpt: {
        |      create: [{c: "c1", c_1: "c", c_2: "1"},{c: "c2", c_1: "c", c_2: "2"}]
        |    }
        |  }){
        |    ${t.parent.selection}
        |    childrenOpt {
        |      ${t.child.selection}
        |    }
        |  }
        |}""",
        project
      )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      val childIdentifier = t.child.where(
        server.query(
          s"""query {
                   |  child(where: {c: "c1"}){
                   |    ${t.child.selection}
                   |  }
                   |}""",
          project
        ),
        "data.child"
      )

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: $parentIdentifier
         |    data:{
         |      childrenOpt: {delete: $childIdentifier}
         |    }
         |  ){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

    }
  }

  "a P1 to C1!  relation " should "work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)
      val parentIdentifier = t.parent.where(
        server.query(
          s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1",
        |    childOpt: {
        |      create: {c: "c1", c_1: "c", c_2: "1"}
        |    }
        |  }){
        |    ${t.parent.selection}
        |  }
        |}""",
          project
        ),
        "data.createParent"
      )

      server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childOpt: {delete: true}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

    }
  }

  "a PM to C1 " should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1",
          |    childrenOpt: {
          |      create: [{c: "c1", c_1: "c", c_2: "1"}, {c: "c2", c_1: "c", c_2: "2"}]
          |    }
          |  }){
          |    ${t.parent.selection}
          |  }
          |}""",
        project
      )

      val parentIdentifier = t.parent.where(res, "data.createParent")
      val childIdentifier = t.child.where(
        server.query(
          s"""query {
             |  child(where: {c: "c1"}){
             |    ${t.child.selection}
             |  }
             |}""",
          project
        ),
        "data.child"
      )

      val res2 = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childrenOpt: {delete: [$childIdentifier]}
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c2"}]}}}""")
    }
  }

  "a P1! to CM  relation" should "error " in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1",
        |    childReq: {
        |      create: {c: "c1", c_1: "c", c_2: "1"}
        |    }
        |  }){
        |    ${t.parent.selection}
        |  }
        |}""",
        project
      )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childReq: {delete: true}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2009,
        errorContains =
          """`Field does not exist on enclosing type.` at `Mutation.updateParent.data.ParentUpdateInput.childReq.ChildUpdateOneRequiredWithoutParentsOptInput.delete`"""
      )
    }
  }

  "a P1 to CM  relation " should "work" taggedAs (IgnoreMsSql) in  {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1",
        |    childOpt: {
        |      create: {c: "c1", c_1: "c", c_2: "1"}
        |    }
        |  }){
        |    ${t.parent.selection}
        |  }
        |}""",
        project
      )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      val res2 = server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: $parentIdentifier
         |    data:{
         |    childOpt: {delete: true}
         |  }){
         |    childOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childOpt":null}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[]}}""")

    }
  }

  "a PM to CM  relation" should "work" taggedAs (IgnoreMongo) in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "otherParent", p_1: "p", p_2: "1",
        |    childrenOpt: {
        |      create: [{c: "otherChild", c_1: "c", c_2: "1"}]
        |    }
        |  }){
        |    childrenOpt{
        |       ${t.child.selection}
        |    }
        |  }
        |}""",
        project
      )

      val childIdentifier = t.child.whereFirst(res, "data.createParent.childrenOpt")

      val res2 = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p2", p_1: "p", p_2: "2",
        |    childrenOpt: {
        |      create: [{c: "c2", c_1: "c", c_2: "2"},{c: "c3", c_1: "c", c_2: "3"},{c: "c4", c_1: "c", c_2: "4"}]
        |    }
        |  }){
        |  ${t.parent.selection}
        |  }
        |}""",
        project
      )

      val parentIdentifier2 = t.parent.where(res2, "data.createParent")

      val child = server.query(
        s"""query {
           |  child(where: {c: "c2"}){
           |    ${t.child.selection}
           |  }
           |}""",
        project
      )

      val childIdentifier2 = t.child.where(child, "data.child")

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier2
         |  data:{
         |    childrenOpt: {delete: [$childIdentifier2, $childIdentifier]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2017,
        errorContains = """The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected."""
      )

      val res3 = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier2
         |  data:{
         |    childrenOpt: {delete: [$childIdentifier2]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res3.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c3"},{"c":"c4"}]}}}""")

      server.query(s"""query{child(where:{c:"c4"}){c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"child":{"c":"c4","parentsOpt":[{"p":"p2"}]}}}""")

      server.query(s"""query{child(where:{c:"otherChild"}){c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"child":{"c":"otherChild","parentsOpt":[{"p":"otherParent"}]}}}""")

    }
  }

  "a PM to CM  relation" should "error on invalid child" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "otherParent", p_1: "p", p_2: "1",
        |    childrenOpt: {
        |      create: [{c: "otherChild", c_1: "c", c_2: "1"}]
        |    }
        |  }){
        |    childrenOpt{
        |       ${t.child.selection}
        |    }
        |  }
        |}""",
        project
      )

      val childIdentifier = t.child.whereFirst(res, "data.createParent.childrenOpt")

      val res2 = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p2", p_1: "p", p_2: "2",
        |    childrenOpt: {
        |      create: [{c: "c2", c_1: "c", c_2: "2"},{c: "c3", c_1: "c", c_2: "3"},{c: "c4", c_1: "c", c_2: "4"}]
        |    }
        |  }){
        |    ${t.parent.selection}
        |  }
        |}""",
        project
      )

      val parentIdentifier = t.parent.where(res2, "data.createParent")

      val childIdentifier2 = t.child.where(
        server.query(
          s"""query {
             |  child(where: {c: "c2"}){
             |    ${t.child.selection}
             |  }
             |}""",
          project
        ),
        "data.child"
      )

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childrenOpt: {delete: [$childIdentifier, $childIdentifier2]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2017,
        errorContains = """The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected"""
      )
    }
  }

  "a PM to CM  relation" should "work for correct children" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "otherParent", p_1: "p", p_2: "1",
        |    childrenOpt: {
        |      create: [{c: "otherChild", c_1: "c", c_2: "1"}]
        |    }
        |  }){
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""",
        project
      )

      val res = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p2", p_1: "p", p_2: "2",
        |    childrenOpt: {
        |      create: [{c: "c2", c_1: "c", c_2: "2"},{c: "c3", c_1: "c", c_2: "3"},{c: "c4", c_1: "c", c_2: "4"}]
        |    }
        |  }){
        |    ${t.parent.selection}
        |  }
        |}""",
        project
      )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      val childIdentifier2 = t.child.where(
        server.query(
          s"""query {
             |  child(where: {c: "c2"}){
             |    ${t.child.selection}
             |  }
             |}""",
          project
        ),
        "data.child"
      )

      val childIdentifier3 = t.child.where(
        server.query(
          s"""query {
             |  child(where: {c: "c3"}){
             |    ${t.child.selection}
             |  }
             |}""",
          project
        ),
        "data.child"
      )

      val res2 = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childrenOpt: {delete: [$childIdentifier2, $childIdentifier3]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c4"}]}}}""")

      server.query(s"""query{parents {p, childrenOpt{c}}}""", project).toString should be(
        """{"data":{"parents":[{"p":"otherParent","childrenOpt":[{"c":"otherChild"}]},{"p":"p2","childrenOpt":[{"c":"c4"}]}]}}""")

      server.query(s"""query{child(where:{c:"c4"}){c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"child":{"c":"c4","parentsOpt":[{"p":"p2"}]}}}""")

      server.query(s"""query{child(where:{c:"otherChild"}){c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"child":{"c":"otherChild","parentsOpt":[{"p":"otherParent"}]}}}""")

    }
  }

  // OTHER DATAMODELS

  "a PM to CM relation" should "delete fail if other req relations would be violated" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Parent{
        | id          String @id @default(cuid())
        | p           String @unique
        | childrenOpt Child[] $relationInlineAttribute
        |}
        |
        |model Child{
        | id         String   @id @default(cuid())
        | c          String   @unique
        | parentsOpt Parent[]
        | otherReq   Other    @relation(references: [id])
        |}
        |
        |model Other{
        | id       String @id @default(cuid())
        | o        String @unique
        | childReq Child
        |}
      """
    }

    database.setup(project)

    server.query(
      """mutation {
        |  createOther(data: {
        |    o: "o1"
        |    childReq: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |    o
        |  }
        |}""",
      project
    )

    server.query(
      """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    childrenOpt: {
        |      connect: {c: "c1"}
        |    }
        |  }){
        |    p
        |  }
        |}""",
      project
    )

    server.queryThatMustFail(
      s"""
         |mutation {
         |  updateParent(
         |  where: { p: "p1"}
         |  data:{
         |    childrenOpt: {delete: [{c: "c1"}]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 2014,
      errorContains = """The change you are trying to make would violate the required relation 'ChildToOther' between the `Child` and `Other` models."""
    )

  }

  "a PM to CM  relation" should "delete the child from other relations as well" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Parent{
        | id          String  @id @default(cuid())
        | p           String  @unique
        | childrenOpt Child[] $relationInlineAttribute
        |}
        |
        |model Child{
        | id         String   @id @default(cuid())
        | c          String   @unique
        | parentsOpt Parent[]
        | otherOpt   Other?   @relation(references: [id])
        |}
        |
        |model Other{
        | id       String @id @default(cuid())
        | o        String @unique
        | childOpt Child?
        |}
      """
    }

    database.setup(project)

    server.query(
      """mutation {
        |  createOther(data: {
        |    o: "o1"
        |    childOpt: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |    o
        |  }
        |}""",
      project
    )

    server.query(
      """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    childrenOpt: {
        |      connect: {c: "c1"}
        |    }
        |  }){
        |    p
        |  }
        |}""",
      project
    )

    val res = server.query(
      s"""
         |mutation {
         |  updateParent(
         |  where: { p: "p1"}
         |  data:{
         |    childrenOpt: {delete: [{c: "c1"}]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
      project
    )

    res.toString should be("""{"data":{"updateParent":{"childrenOpt":[]}}}""")

    server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[]}}""")

  }

  "a one to many relation" should "be deletable by id through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Todo{
        | id        String    @id @default(cuid())
        | comments  Comment[] $relationInlineAttribute
        |}
        |
        |model Comment{
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo?
        |}
      """
    }

    database.setup(project)

    val otherCommentId = server
      .query(
        """mutation {
        |  createComment(
        |    data: {
        |      text: "otherComment"
        |    }
        |  ){
        |    id
        |  }
        |}""",
        project
      )
      .pathAsString("data.createComment.id")

    val createResult = server.query(
      """mutation {
        |  createTodo(
        |    data: {
        |      comments: {
        |        create: [{text: "comment1"}, {text: "comment2"}]
        |      }
        |    }
        |  ){
        |    id
        |    comments { id }
        |  }
        |}""",
      project
    )

    val todoId     = createResult.pathAsString("data.createTodo.id")
    val comment1Id = createResult.pathAsString("data.createTodo.comments.[0].id")
    val comment2Id = createResult.pathAsString("data.createTodo.comments.[1].id")

    val result = server.query(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        delete: [{id: "$comment1Id"}, {id: "$comment2Id"}]
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

    server.queryThatMustFail(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        delete: [{id: "$otherCommentId"}]
         |      }
         |    }
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 2017,
      errorContains = """The records for relation `CommentToTodo` between the `Todo` and `Comment` models are not connected."""
    )

    mustBeEqual(result.pathAsJsValue("data.updateTodo.comments").toString, """[]""")

    val query = server.query("""{ comments { text }}""", project)
    mustBeEqual(query.toString, """{"data":{"comments":[{"text":"otherComment"}]}}""")

  }

  "a one to many relation" should "be deletable by any unique argument through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Todo{
        | id       String    @id @default(cuid())
        | comments Comment[] $relationInlineAttribute
        |}
        |
        |model Comment{
        | id    String  @id @default(cuid())
        | text  String?
        | alias String  @unique
        | todo  Todo?
        |}
      """
    }

    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createTodo(
        |    data: {
        |      comments: {
        |        create: [{text: "comment1", alias: "alias1"}, {text: "comment2", alias: "alias2"}]
        |      }
        |    }
        |  ){
        |    id
        |    comments { id }
        |  }
        |}""",
      project
    )
    val todoId = createResult.pathAsString("data.createTodo.id")

    val result = server.query(
      s"""mutation {
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        delete: [{alias: "alias1"}, {alias: "alias2"}]
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

    mustBeEqual(result.pathAsJsValue("data.updateTodo.comments").toString, """[]""")

    val query = server.query("""{ comments { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"comments":[]}}""")

  }

  "a many to one relation" should "be deletable by id through a nested mutation" ignore {

    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Todo{
        | id       String    @id @default(cuid())
        | comments Comment[] $relationInlineAttribute
        |}
        |
        |model Comment{
        | id   String @id @default(cuid())
        | text String?
        | todo Todo?
        |}
      """
    }

    database.setup(project)

    val existingCreateResult = server.query(
      """mutation {
        |  createTodo(
        |    data: {
        |      comments: {
        |        create: [{text: "otherComment"}]
        |      }
        |    }
        |  ){
        |    id
        |    comments { id }
        |  }
        |}""",
      project
    )
    val existingTodoId    = existingCreateResult.pathAsString("data.createTodo.id")
    val existingCommentId = existingCreateResult.pathAsString("data.createTodo.comments.[0].id")

    val createResult = server.query(
      """mutation {
        |  createTodo(
        |    data: {
        |      comments: {
        |        create: [{text: "comment1"}]
        |      }
        |    }
        |  ){
        |    id
        |    comments { id }
        |  }
        |}""",
      project
    )
    val todoId    = createResult.pathAsString("data.createTodo.id")
    val commentId = createResult.pathAsString("data.createTodo.comments.[0].id")

    val result = server.query(
      s"""
         |mutation {
         |  updateComment(
         |    where: {
         |      id: "$commentId"
         |    }
         |    data: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |  ){
         |    todo {
         |      id
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsJsValue("data.updateComment").toString, """{"todo":null}""")

    val query = server.query("""{ todoes { id comments { id } }}""", project)
    mustBeEqual(query.toString, s"""{"data":{"todoes":[{"id":"$existingTodoId","comments":[{"id":"$existingCommentId"}]}]}}""")
  }

  "one2one relation both exist and are connected" should "be deletable by id through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo{
        | id    String  @id @default(cuid())
        | title String?
        | note  Note?   @relation(references: [id])
        |}
        |
        |model Note{
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo?
        |}
      """
    }

    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createNote(
        |    data: {
        |      todo: {
        |        create: { title: "the title" }
        |      }
        |    }
        |  ){
        |    id
        |    todo { id }
        |  }
        |}""",
      project
    )
    val noteId = createResult.pathAsString("data.createNote.id")
    val todoId = createResult.pathAsString("data.createNote.todo.id")

    val result = server.query(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    data: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |  ){
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsJsValue("data.updateNote").toString, """{"todo":null}""")

    val query = server.query("""{ todoes { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")
  }

  "one2one relation both exist and are connected" should "be deletable by unique field through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo{
        | id    String @id @default(cuid())
        | title String @unique
        | note  Note?  @relation(references: [id])
        |}
        |
        |model Note{
        | id   String @id @default(cuid())
        | text String @unique
        | todo Todo?
        |}
      """
    }
    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createNote(
        |    data: {
        |      text: "FirstUnique"
        |      todo: {
        |        create: { title: "the title" }
        |      }
        |    }
        |  ){
        |    id
        |  }
        |}""",
      project
    )

    val result = server.query(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      text: "FirstUnique"
         |    }
         |    data: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |  ){
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateNote").toString, """{"todo":null}""")

    val query = server.query("""{ todoes { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")

    val query2 = server.query("""{ notes { text }}""", project)
    mustBeEqual(query2.toString, """{"data":{"notes":[{"text":"FirstUnique"}]}}""")
  }

  "a one to one relation" should "not do a nested delete by id if the nested node does not exist" ignore {
    val project = SchemaDsl.fromStringV11() {
      """
        |model Todo{
        | id    String  @id @default(cuid())
        | title String?
        | note  Note?   @relation(references: [id])
        |}
        |
        |model Note{
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo?
        |}
      """
    }
    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createNote(
        |    data: {
        |      text: "Note"
        |    }
        |  ){
        |    id
        |    todo { id }
        |  }
        |}""",
      project
    )
    val noteId = createResult.pathAsString("data.createNote.id")

    val result = server.queryThatMustFail(
      s"""
         |mutation {
         |  updateNote(
         |    where: {id: "$noteId"}
         |    data: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |  ){
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 2016,
      errorContains = """"[Query Graph] Expected a valid parent ID to be present for a nested delete on a one-to-many relation."""
    )

    val query = server.query("""{ todoes { title }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")

    val query2 = server.query("""{ notes { text }}""", project)
    mustBeEqual(query2.toString, """{"data":{"notes":[{"text":"Note"}]}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String @id @default(cuid())
                                             |  nameTop String @unique
                                             |  middles Middle[] $relationInlineAttribute
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String @id @default(cuid())
                                             |  nameMiddle String @unique
                                             |  tops       Top[]
                                             |  bottoms    Bottom[] $relationInlineAttribute
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String   @id @default(cuid())
                                             |  nameBottom String   @unique
                                             |  middles    Middle[]
                                             |}""" }
    database.setup(project)

    val createMutation =
      """
        |mutation  {
        |  createTop(data: {
        |    nameTop: "the top",
        |    middles: {
        |      create:[
        |        {
        |          nameMiddle: "the middle"
        |          bottoms: {
        |            create: [{ nameBottom: "the bottom"}, { nameBottom: "the second bottom"}]
        |          }
        |        },
        |        {
        |          nameMiddle: "the second middle"
        |          bottoms: {
        |            create: [{nameBottom: "the third bottom"},{nameBottom: "the fourth bottom"}]
        |          }
        |        }
        |     ]
        |    }
        |  }) {id}
        |}
      """

    server.query(createMutation, project)

    val updateMutation =
      s"""
         |mutation b {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middles: {
         |        update: [
         |          {
         |            where: { nameMiddle: "the middle" }
         |            data: {
         |              nameMiddle: { set: "updated middle" }
         |              bottoms: { delete: [{ nameBottom: "the bottom" }] }
         |            }
         |          }
         |        ]
         |      }
         |    }
         |  ) {
         |    nameTop
         |    middles(orderBy: { id: asc }) {
         |      nameMiddle
         |      bottoms(orderBy: { id: asc }) {
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottoms":[{"nameBottom":"the second bottom"}]},{"nameMiddle":"the second middle","bottoms":[{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}]}}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path and there are no backrelations" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String   @id @default(cuid())
                                             |  nameTop String   @unique
                                             |  middles Middle[] $relationInlineAttribute
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String   @id @default(cuid())
                                             |  nameMiddle String   @unique
                                             |  bottoms    Bottom[] $relationInlineAttribute
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String @id @default(cuid())
                                             |  nameBottom String @unique
                                             |}""" }
    database.setup(project)

    val createMutation =
      """
        |mutation  {
        |  createTop(data: {
        |    nameTop: "the top",
        |    middles: {
        |      create:[
        |        {
        |          nameMiddle: "the middle"
        |          bottoms: {
        |            create: [{ nameBottom: "the bottom"}, { nameBottom: "the second bottom"}]
        |          }
        |        },
        |        {
        |          nameMiddle: "the second middle"
        |          bottoms: {
        |            create: [{nameBottom: "the third bottom"},{nameBottom: "the fourth bottom"}]
        |          }
        |        }
        |     ]
        |    }
        |  }) {id}
        |}
      """

    server.query(createMutation, project)

    val updateMutation =
      s"""
         |mutation b {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middles: {
         |        update: [
         |          {
         |            where: { nameMiddle: "the middle" }
         |            data: {
         |              nameMiddle: { set: "updated middle" }
         |              bottoms: { delete: [{ nameBottom: "the bottom" }] }
         |            }
         |          }
         |        ]
         |      }
         |    }
         |  ) {
         |    nameTop
         |    middles(orderBy: { id: asc }) {
         |      nameMiddle
         |      bottoms {
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottoms":[{"nameBottom":"the second bottom"}]},{"nameMiddle":"the second middle","bottoms":[{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}]}}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are model and node edges on the path " ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String @id @default(cuid())
                                             |  nameTop String @unique
                                             |  middles Middle[] $relationInlineAttribute
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String  @id @default(cuid())
                                             |  nameMiddle String  @unique
                                             |  tops       Top[]
                                             |  bottom     Bottom? @relation(references: [id])
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String  @id @default(cuid())
                                             |  nameBottom String  @unique
                                             |  middle     Middle?
                                             |}""" }
    database.setup(project)

    val createMutation =
      """
        |mutation  {
        |  createTop(data: {
        |    nameTop: "the top",
        |    middles: {
        |      create:[
        |        {
        |          nameMiddle: "the middle"
        |          bottom: {create: { nameBottom: "the bottom"}}
        |        },
        |        {
        |          nameMiddle: "the second middle"
        |          bottom: {create: { nameBottom: "the second bottom"}}
        |        }
        |     ]
        |    }
        |  }) {id}
        |}
      """

    server.query(createMutation, project)

    val updateMutation =
      s"""
         |mutation b {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middles: {
         |        update: [
         |          {
         |            where: { nameMiddle: "the middle" }
         |            data: {
         |              nameMiddle: { set: "updated middle" }
         |              bottom: { delete: true }
         |            }
         |          }
         |        ]
         |      }
         |    }
         |  ) {
         |    nameTop
         |    middles(orderBy: { id: asc }) {
         |      nameMiddle
         |      bottom {
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottom":null},{"nameMiddle":"the second middle","bottom":{"nameBottom":"the second bottom"}}]}}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are model and node edges on the path  and back relations are missing and node edges follow model edges" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String  @id @default(cuid())
                                             |  nameTop String  @unique
                                             |  middle  Middle? @relation(references: [id])
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String  @id @default(cuid())
                                             |  nameMiddle String  @unique
                                             |  bottom     Bottom? @relation(references: [id])
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String  @id @default(cuid())
                                             |  nameBottom String  @unique
                                             |  below      Below[] $relationInlineAttribute
                                             |}
                                             |
                                             |model Below {
                                             |  id        String @id @default(cuid())
                                             |  nameBelow String @unique
                                             |}""" }
    database.setup(project)

    val createMutation =
      """
        |mutation a {
        |  createTop(data: {
        |    nameTop: "the top",
        |    middle: {
        |      create:
        |        {
        |          nameMiddle: "the middle"
        |          bottom: {
        |            create: { nameBottom: "the bottom"
        |            below: {
        |            create: [{ nameBelow: "below"}, { nameBelow: "second below"}]}
        |        }}}
        |        }
        |  }) {id}
        |}
      """

    server.query(createMutation, project)

    val updateMutation =
      s"""
         |mutation b {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middle: {
         |        update: {
         |          nameMiddle: { set: "updated middle" }
         |          bottom: {
         |            update: {
         |              nameBottom: { set: "updated bottom" }
         |              below: { delete: { nameBelow: "below" } }
         |            }
         |          }
         |        }
         |      }
         |    }
         |  ) {
         |    nameTop
         |    middle {
         |      nameMiddle
         |      bottom {
         |        nameBottom
         |        below {
         |          nameBelow
         |        }
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom","below":[{"nameBelow":"second below"}]}}}}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only model edges on the path" ignore {
    val project = SchemaDsl.fromStringV11() { """model Top {
                                             |  id      String  @id @default(cuid())
                                             |  nameTop String  @unique
                                             |  middle  Middle? @relation(references: [id])
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String @id @default(cuid())
                                             |  nameMiddle String @unique
                                             |  top        Top?
                                             |  bottom     Bottom? @relation(references: [id])
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String  @id @default(cuid())
                                             |  middle     Middle?
                                             |  nameBottom String  @unique
                                             |}""" }
    database.setup(project)

    val createMutation =
      """
        |mutation  {
        |  createTop(data: {
        |    nameTop: "the top",
        |    middle: {
        |      create:
        |        {
        |          nameMiddle: "the middle"
        |          bottom: {
        |            create: {
        |              nameBottom: "the bottom"
        |            }
        |          }
        |        }
        |    }
        |  }) {id}
        |}
      """

    server.query(createMutation, project)

    val updateMutation =
      s"""
         |mutation {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middle: {
         |        update: {
         |          nameMiddle: { set: "updated middle" }
         |          bottom: { delete: true }
         |        }
         |      }
         |    }
         |  ) {
         |    nameTop
         |    middle {
         |      nameMiddle
         |      bottom {
         |        nameBottom
         |      }
         |    }
         |  }
         |}
         |
      """

    val result = server.query(updateMutation, project)

    result.toString should be("""{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":null}}}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only model edges on the path and there are no backrelations" ignore {
    val project = SchemaDsl.fromStringV11() { """model Top {
                                             |  id      String @id @default(cuid())
                                             |  nameTop String @unique
                                             |  middle  Middle? @relation(references: [id])
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String  @id @default(cuid())
                                             |  nameMiddle String  @unique
                                             |  bottom     Bottom? @relation(references: [id])
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String @id @default(cuid())
                                             |  nameBottom String @unique
                                             |}""" }
    database.setup(project)

    val createMutation =
      """
        |mutation  {
        |  createTop(data: {
        |    nameTop: "the top",
        |    middle: {
        |      create:
        |        {
        |          nameMiddle: "the middle"
        |          bottom: {
        |            create: {
        |              nameBottom: "the bottom"
        |            }
        |          }
        |        }
        |    }
        |  }) {id}
        |}
      """

    server.query(createMutation, project)

    val updateMutation =
      s"""
         |mutation {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middle: {
         |        update: {
         |          nameMiddle: { set: "updated middle" }
         |          bottom: { delete: true }
         |        }
         |      }
         |    }
         |  ) {
         |    nameTop
         |    middle {
         |      nameMiddle
         |      bottom {
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be("""{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":null}}}}""")
  }

  "Nested delete on self relations" should "only delete the specified nodes" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model User {
                                             |  id        String @id @default(cuid())
                                             |  name      String @unique
                                             |  follower  User[] @relation(name: "UserFollow" $listInlineArgument)
                                             |  following User[] @relation(name: "UserFollow")
                                             |}""" }
    database.setup(project)

    server.query("""mutation  {createUser(data: {name: "X"}) {id}}""", project)
    server.query("""mutation  {createUser(data: {name: "Y"}) {id}}""", project)
    server.query("""mutation  {createUser(data: {name: "Z"}) {id}}""", project)

    val updateMutation =
      s"""
         |mutation {
         |  updateUser(
         |    data: { following: { connect: [{ name: "Y" }, { name: "Z" }] } }
         |    where: { name: "X" }
         |  ) {
         |    name
         |    following {
         |      name
         |    }
         |    follower {
         |      name
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be("""{"data":{"updateUser":{"name":"X","following":[{"name":"Y"},{"name":"Z"}],"follower":[]}}}""")

    val check = server.query("""query{users{name, following{name}}}""", project)

    check.toString should be(
      """{"data":{"users":[{"name":"X","following":[{"name":"Y"},{"name":"Z"}]},{"name":"Y","following":[]},{"name":"Z","following":[]}]}}""")

    val deleteMutation =
      s"""
         |mutation {
         |  updateUser(
         |    data: { follower: { delete: [{ name: "X" }] } }
         |    where: { name: "Y" }
         |  ) {
         |    name
         |    following {
         |      name
         |    }
         |  }
         |}
         |
      """

    val result2 = server.query(deleteMutation, project)

    result2.toString should be("""{"data":{"updateUser":{"name":"Y","following":[]}}}""")

    val result3 = server.query("""query{users{name, following{name}}}""", project)

    result3.toString should be("""{"data":{"users":[{"name":"Y","following":[]},{"name":"Z","following":[]}]}}""")
  }
}
