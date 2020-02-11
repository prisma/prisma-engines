package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedSetMutationInsideUpdateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "a PM to C1  relation with the child already in a relation" should "be setable through a nested mutation by unique" in {
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

      // we are even resilient against multiple identical connects here -> twice connecting to c2

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: { p: "p2"}
         |  data:{
         |    childrenOpt: {set: [{c: "c1"},{c: "c2"},{c: "c2"}]}
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

  "a PM to C1  relation with the child without a relation" should "be setable through a nested mutation by unique" in {
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

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: {p:"p1"}
         |  data:{
         |    childrenOpt: {set: {c: "c1"}}
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

    }
  }

  "a PM to CM  relation with the children already in a relation" should "be setable through a nested mutation by unique" in {
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

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: {    p: "p2"}
         |  data:{
         |    childrenOpt: {set: [{c: "c1"}, {c: "c2"}]}
         |  }){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c2","parentsOpt":[{"p":"p1"},{"p":"p2"}]},{"c":"c3","parentsOpt":[]},{"c":"c4","parentsOpt":[]}]}}""")

    }
  }

  "a PM to CM  relation with the child not already in a relation" should "be setable through a nested mutation by unique" in {
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

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: {p: "p1"}
         |  data:{
         |    childrenOpt: {set: {c: "c1"}}
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

    }
  }

  "a PM to CM  relation with the children already in a relation" should "be setable to empty" in {
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

      val res = server.query(
        s"""
         |mutation {
         |  updateParent(
         |  where: {    p: "p2"}
         |  data:{
         |    childrenOpt: {set: []}
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

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c3","parentsOpt":[]},{"c":"c4","parentsOpt":[]}]}}""")

    }
  }

  "a one to many relation" should "be setable by id through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment {
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo?   @relation(references: [id])
        |}
        |
        |model Todo {
        | id       String    @id @default(cuid())
        | comments Comment[]
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
         |        set: [{id: "$comment1Id"}, {id: "$comment2Id"}]
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

  "a one to many relation" should "be setable by unique through a nested mutation" in {
    val project = SchemaDsl.fromStringV11() {
      """model Comment {
        | id   String  @id @default(cuid())
        | text String? @unique
        | todo Todo?   @relation(references: [id])
        |}
        |
        |model Todo {
        | id       String    @id @default(cuid())
        | title    String?   @unique
        | comments Comment[]
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
         |        set: [{text: "comment1"}, {text: "comment2"}]
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

  "a PM to CM  self relation with the child not already in a relation" should "be setable through a nested mutation by unique" in {
    val project = SchemaDsl.fromStringV11() {
      s"""model Technology {
         |  id                 String       @id @default(cuid())
         |  name               String       @unique
         |  childTechnologies  Technology[] @relation(name: "ChildTechnologies" $listInlineArgument)
         |  parentTechnologies Technology[] @relation(name: "ChildTechnologies")
        |}
      """.stripMargin
    }
    database.setup(project)

    server.query("""mutation {createTechnology(data: {name: "techA"}){name}}""", project)
    server.query("""mutation {createTechnology(data: {name: "techB"}){name}}""", project)

    val res = server.query(
      s"""mutation {
         |  updateTechnology(where: {name: "techA"},
         |                   data:  {childTechnologies: {set: {name: "techB"}}})
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

  "Setting two nodes twice" should "not error" in {
    val project = SchemaDsl.fromStringV11() {
      s"""model Child {
        | id      String   @id @default(cuid())
        | c       String   @unique
        | parents Parent[] $relationInlineDirective
        |}
        |
        |model Parent {
        | id       String  @id @default(cuid())
        | p        String  @unique
        | children Child[]
        |}
      """.stripMargin
    }
    database.setup(project)

    val parentId = server
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

    val childId = server
      .query(
        """mutation {
          |  createParent(data: {
          |    p: "p2"
          |    children: {
          |      create: {c: "c1"}
          |    }
          |  }){
          |    children{id}
          |  }
          |}""",
        project
      )
      .pathAsString("data.createParent.children.[0].id")

    val res = server.query(
      s"""
         |mutation {
         |  updateParent(
         |  where:{id: "$parentId"}
         |  data:{
         |    children: {set: {id: "$childId"}}
         |  }){
         |    children {
         |      c
         |    }
         |  }
         |}
      """,
      project
    )

    res.toString should be("""{"data":{"updateParent":{"children":[{"c":"c1"}]}}}""")

    server.query(
      s"""
         |mutation {
         |  updateParent(
         |  where:{id: "$parentId"}
         |  data:{
         |    children: {set: {id: "$childId"}}
         |  }){
         |    children {
         |      c
         |    }
         |  }
         |}
      """,
      project
    )

  }

  "Setting several times" should "not error and only connect the item once" in {

    val project = SchemaDsl.fromStringV11() {
      s"""
        |model Post {
        |  id      String  @id @default(cuid())
        |  authors AUser[] $relationInlineDirective
        |  title   String  @unique
        |}
        |
        |model AUser {
        |  id    String @id @default(cuid())
        |  name  String @unique
        |  posts Post[]
        |}"""
    }

    database.setup(project)

    server.query(s""" mutation {createPost(data: {title:"Title"}) {title}} """, project)
    server.query(s""" mutation {createAUser(data: {name:"Author"}) {name}} """, project)

    server.query(s""" mutation {updateAUser(where: { name: "Author"}, data:{posts:{set:{title: "Title"}}}) {name}} """, project)
    server.query(s""" mutation {updateAUser(where: { name: "Author"}, data:{posts:{set:{title: "Title"}}}) {name}} """, project)
    server.query(s""" mutation {updateAUser(where: { name: "Author"}, data:{posts:{set:{title: "Title"}}}) {name}} """, project)

    server.query("""query{aUsers{name, posts{title}}}""", project).toString should be("""{"data":{"aUsers":[{"name":"Author","posts":[{"title":"Title"}]}]}}""")
  }
}
