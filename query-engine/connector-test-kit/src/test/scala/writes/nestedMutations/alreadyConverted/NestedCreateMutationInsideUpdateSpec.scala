package writes.nestedMutations.alreadyConverted

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedCreateMutationInsideUpdateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  //todo which tests to keep and which ones to delete???? Some do not really test the compound unique functionality

  "a P1! to C1! relation" should "error since old required parent relation would be broken" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1"
          |    childReq: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    ${t.parent.selection}
          |    childReq {
          |      ${t.child.selection}
          |    }
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
         |    p: "p2"
         |    childReq: {create: {c: "SomeC"}}
         |  }){
         |  p
         |  childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 3042,
        errorContains = "The change you are trying to make would violate the required relation 'ChildToParent' between Child and Parent"
      )

    }
  }

  "a P1! to C1 relation" should "work" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1"
          |    childReq: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |   ${t.parent.selection}
          |   childReq {
          |     ${t.child.selection}
          |   }
          |  }
          |}""",
          project
        )

      val parentIdentifier = t.parent.where(res, "data.createParent")
//      println(s"parentIdentifier: ${parentIdentifier}")

      val res2 = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    p: "p2"
         |    childReq: {create: {c: "SomeC"}}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childReq":{"c":"SomeC"}}}}""")

    }
  }

  "a P1 to C1  relation " should "work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1"
          |    childOpt: {
          |      create: {c: "c1"}
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
         |  where: $parentIdentifier
         |  data:{
         |    p: "p2"
         |    childOpt: {create: {c: "SomeC"}}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"SomeC"}}}}""")

    }
  }

  "a P1 to C1  relation with the parent without a relation" should "work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parent = server
        .query(
          s"""mutation {
          |  createParent(data: {p: "p1", p_1: "p", p_2: "1"})
          |  {
          |    ${t.parent.selection}
          |  }
          |}""",
          project
        )

      val parentIdentifier = t.parent.where(parent, "data.createParent")

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    p: "p2"
         |    childOpt: {create: {c: "SomeC"}}
         |  }){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"SomeC"}}}}""")

    }
  }

  "a PM to C1!  relation with a child already in a relation" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1"
        |    childrenOpt: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |    ${t.parent.selection}
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""",
        project
      )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: $parentIdentifier
         |    data:{
         |      childrenOpt: {create: {c: "c2"}}
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

    }
  }

  "a P1 to C1!  relation with the parent and a child already in a relation" should "error in a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1"
        |    childOpt: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |     ${t.parent.selection}
        |    childOpt{
        |       c
        |    }
        |  }
        |}""",
        project
      )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      server.queryThatMustFail(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childOpt: {create: {c: "c2"}}
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
    }
  }

  "a P1 to C1!  relation with the parent not already in a relation" should "work in a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server.query(
        s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1"
          |  }){
          |    ${t.parent.selection}
          |    p
          |  }
          |}""",
        project
      )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      val res = server.query(
        s"""
           |mutation {
           |  updateParent(
           |  where: $parentIdentifier
           |  data:{
           |    childOpt: {create: {c: "c1"}}
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

    }
  }

  "a PM to C1  relation with the parent already in a relation" should "work through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1", p_1: "p", p_2: "1"
          |    childrenOpt: {
          |      create: [{c: "c1"}, {c: "c2"}]
          |    }
          |  }){
          |    ${t.parent.selection}
          |    childrenOpt{
          |       c
          |    }
          |  }
          |}""",
          project
        )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childrenOpt: {create: [{c: "c3"}]}
         |  }){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c3"}]}}}""")

    }
  }

  "a P1! to CM  relation with the parent already in a relation" should "work through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1"
        |    childReq: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |    ${t.parent.selection}
        |    childReq{
        |       c
        |    }
        |  }
        |}""",
        project
      )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childReq: {create: {c: "c2"}}
         |  }){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childReq":{"c":"c2"}}}}""")

    }
  }

  "a P1 to CM  relation with the child already in a relation" should "work through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1"
        |    childOpt: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |    ${t.parent.selection}
        |    childOpt{
        |       c
        |    }
        |  }
        |}""",
        project
      )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |    where: $parentIdentifier
         |    data:{
         |    childOpt: {create: {c: "c2"}}
         |  }){
         |    childOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childOpt":{"c":"c2"}}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[]},{"c":"c2","parentsOpt":[{"p":"p1"}]}]}}""")

    }
  }

  "a PM to CM  relation with the children already in a relation" should "be disconnectable through a nested mutation by unique" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val parentResult = server.query(
        s"""mutation {
        |  createParent(data: {
        |    p: "p1", p_1: "p", p_2: "1"
        |    childrenOpt: {
        |      create: [{c: "c1"},{c: "c2"}]
        |    }
        |  }){
        |    ${t.parent.selection}
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""",
        project
      )
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: $parentIdentifier
         |  data:{
         |    childrenOpt: {create: [{c: "c3"}]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c3"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c3","parentsOpt":[{"p":"p1"}]}]}}""")

    }
  }

  "a one to many relation" should "be creatable through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment{
        |   id   String  @id @default(cuid())
        |   text String?
        |   todo Todo?   @relation(references: [id])
        |}
        |
        |model Todo{
        |   id       String    @id @default(cuid())
        |   comments Comment[]
        |}"""
    }

    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createTodo(data:{}){
        |    id
        |  }
        |}
      """,
      project
    )
    val id = createResult.pathAsString("data.createTodo.id")

    val result = server.query(
      s"""mutation {
        |  updateTodo(
        |    where: {
        |      id: "$id"
        |    }
        |    data:{
        |      comments: {
        |        create: [{text: "comment1"}, {text: "comment2"}]
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

  "a many to one relation" should "be creatable through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment{
        |   id   String  @id @default(cuid())
        |   text String?
        |   todo Todo?   @relation(references: [id])
        |}
        |
        |model Todo{
        |   id       String  @id @default(cuid())
        |   title    String?
        |   comments Comment[]
        |}"""
    }

    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createComment(data:{}){
        |    id
        |  }
        |}
      """,
      project
    )
    val id = createResult.pathAsString("data.createComment.id")

    val result = server.query(
      s"""
        |mutation {
        |  updateComment(
        |    where: {
        |      id: "$id"
        |    }
        |    data: {
        |      todo: {
        |        create: {title: "todo1"}
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
    mustBeEqual(result.pathAsString("data.updateComment.todo.title"), "todo1")
  }

  "a many to one relation" should "be creatable through a nested mutation using non-id unique field" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment{
        |   id   String @id @default(cuid())
        |   text String @unique
        |   todo Todo?  @relation(references: [id])
        |}
        |
        |model Todo{
        |   id       String @id @default(cuid())
        |   title    String @unique
        |   comments Comment[]
        |}"""
    }

    database.setup(project)

    server.query(
      """mutation {
        |  createComment(data:{ text: "comment"}){
        |    id
        |    text
        |  }
        |}
      """,
      project
    )

    val result = server.query(
      s"""
         |mutation {
         |  updateComment(
         |    where: {
         |      text: "comment"
         |    }
         |    data: {
         |      todo: {
         |        create: {title: "todo1"}
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
    mustBeEqual(result.pathAsString("data.updateComment.todo.title"), "todo1")
  }

}
