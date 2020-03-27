package writes.nestedMutations.alreadyConverted

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedDeleteMutationInsideUpsertSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
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
          |    p: "p1", p_1: "p", p_2: "1"
          |    childReq: {
          |      create: {
          |        c: "c1"
          |        c_1: "c_1"
          |        c_2: "c_2"
          |      }
          |    }
          |  }){
          |    ${t.parent.selection}
          |    childReq{
          |       ${t.child.selection}
          |    }
          |  }
          |}""",
          project
        )
      val childId  = t.child.where(res, "data.createParent.childReq")
      val parentId = t.parent.where(res, "data.createParent")

      server.queryThatMustFail(
        s"""mutation {
         |  upsertParent(
         |  where: $parentId
         |  update:{
         |    p: "p2"
         |    childReq: {delete: true}
         |  }
         |  create:{p: "Should not matter" childReq: {create: {c: "Should not matter"}}}
         |  ){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 0,
        errorContains = "Argument 'update' expected model 'ParentUpdateInput!'"
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
          |    p: "p1", p_1: "p", p_2: "1"
          |    childReq: {
          |      create: {
          |        c: "c1"
          |        c_1: "c_1"
          |        c_2: "c_2"
          |      }
          |    }
          |  }){
          |    ${t.parent.selection}
          |    childReq{
          |       ${t.child.selection}
          |    }
          |  }
          |}""",
          project
        )

      val childId  = t.child.where(res, "data.createParent.childReq")
      val parentId = t.parent.where(res, "data.createParent")

      server.queryThatMustFail(
        s"""mutation {
         |  upsertParent(
         |  where: $parentId
         |  update:{
         |    p: "p2"
         |    childReq: {delete: true}
         |  }
         |  create:{p: "Should not matter" childReq: {create: {c: "Should not matter"}}}
         |  ){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 0,
        errorContains = "Argument 'update' expected model 'ParentUpdateInput!'"
      )

    }
  }

  "a P1 to C1  relation " should "work through a nested mutation by id" in {
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
          |    childOpt{
          |       ${t.child.selection}
          |    }
          |  }
          |}""",
          project
        )

      val childId  = t.child.where(res, "data.createParent.childOpt")
      val parentId = t.parent.where(res, "data.createParent")

      val res2 = server.query(
        s"""
         |mutation {
         |  upsertParent(
         |  where: $parentId
         |  update:{
         |    p: "p2"
         |    childOpt: {delete: true}
         |  }
         |  create:{p: "Should not matter"}
         |  ){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res2.toString should be("""{"data":{"upsertParent":{"childOpt":null}}}""")

    }
  }

  "a P1 to C1  relation" should "error if the nodes are not connected" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Result = server
        .query(
          s"""mutation {
          |  createChild(data: {
          |    c: "c1"
          |    c_1: "c_1"
          |    c_2: "c_2"
          |  })
          |  {
          |    ${t.child.selection}
          |  }
          |}""",
          project
        )
      val child1Id = t.child.where(child1Result, "data.createChild")

      val parent1Result = server
        .query(
          s"""mutation {
          |  createParent(data: {p: "p1", p_1: "p", p_2: "1"})
          |  {
          |    ${t.parent.selection}
          |  }
          |}""",
          project
        )
      val parent1Id = t.parent.where(parent1Result, "data.createParent")

      val res = server.queryThatMustFail(
        s"""mutation {
         |  upsertParent(
         |    where: $parent1Id
         |    update:{
         |      p: "p2"
         |      childOpt: {delete: true}
         |    }
         |    create:{p: "Should not matter"}
         |  ){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 3041
      )

    }
  }

  "a PM to C1!  relation " should "work" in {
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

      server.query(
        s"""
         |mutation {
         |  upsertParent(
         |    where: $parentIdentifier
         |    update:{
         |    childrenOpt: {delete: {c: "c1"}}
         |  }
         |  create:{p: "Should not matter"}
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
      val parentIdentifier1 = t.parent.where(parentResult, "data.createParent")

      server.query(
        s"""
         |mutation {
         |  upsertParent(
         |  where: $parentIdentifier1
         |  update:{
         |    childOpt: {delete: true}
         |  }
         |  create:{p: "Should not matter"}
         |  ){
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
         |  upsertParent(
         |  where: $parentIdentifier
         |  update:{
         |    childrenOpt: {delete: [{c: "c2"}]}
         |  }
         |   create:{p: "Should not matter"}
         |  ){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"upsertParent":{"childrenOpt":[{"c":"c1"}]}}}""")

    }
  }

  "a P1! to CM  relation" should "error" in {
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
        |      create: {
        |        c: "c1"
        |        c_1: "c_1"
        |        c_2: "c_2"
        |      }
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

      server.queryThatMustFail(
        s"""mutation {
         |  upsertParent(
         |  where: $parentIdentifier
         |  update:{
         |    childReq: {delete: true}
         |  }
         |  create:{p: "Should not matter",childReq: {create:{c: "Should not matter"}}}
         |  ){
         |    childReq {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 0,
        errorContains = "Argument 'update' expected model 'ParentUpdateInput!'"
      )
    }
  }

  "a P1 to CM  relation " should "work" in {
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
         |  upsertParent(
         |    where: $parentIdentifier
         |    update:{childOpt: {delete: true}}
         |    create:{p: "Should not matter"}
         |  ){
         |    childOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"upsertParent":{"childOpt":null}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[]}}""")

    }
  }

  "a PM to CM  relation" should "work" in {
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
         |  upsertParent(
         |  where: $parentIdentifier
         |  update:{
         |    childrenOpt: {delete: [{c: "c1"}, {c: "c2"}]}
         |  }
         |  create:{p: "Should not matter"}
         |  ){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"upsertParent":{"childrenOpt":[]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[]}}""")

    }
  }

  "a PM to CM  relation" should "delete fail if other req relations would be violated" ignore {

    val schema = s"""model Parent{
                            id          String  @id @default(cuid())
                            p           String  @unique
                            childrenOpt Child[] $relationInlineDirective
                        }

                        model Child{
                            id         String   @id @default(cuid())
                            c          String   @unique
                            parentsOpt Parent[]
                            otherReq   ReqOther @relation(references: [id])
                        }

                        model ReqOther{
                            id       String @id @default(cuid())
                            r        String @unique
                            childReq Child
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
    database.setup(project)

    server.query(
      """mutation {
        |  createReqOther(data: {
        |    r: "r1"
        |    childReq: {
        |      create: {c: "c1"}
        |    }
        |  }){
        |    r
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
      s"""mutation {
         |  upsertParent(
         |  where: { p: "p1"}
         |  update:{
         |    childrenOpt: {delete: [{c: "c1"}]}
         |  }
         |  create:{p: "Should not matter"}
         |
         |  ){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 3042,
      errorContains = """The change you are trying to make would violate the required relation 'ChildToReqOther' between Child and ReqOther"""
    )

  }

  "a PM to CM  relation" should "delete the child from other relations as well" ignore {
    val schema = s"""model Parent{
                            id          String  @id @default(cuid())
                            p           String  @unique
                            childrenOpt Child[] $relationInlineDirective
                        }

                        model Child{
                            id         String    @id @default(cuid())
                            c          String    @unique
                            parentsOpt Parent[]
                            otherOpt   OptOther? @relation(references: [id])
                        }

                        model OptOther{
                            id       String @id @default(cuid())
                            o        String @unique
                            childOpt Child?
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
    database.setup(project)

    server.query(
      """mutation {
        |  createOptOther(data: {
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
         |  upsertParent(
         |  where: { p: "p1"}
         |  update:{
         |    childrenOpt: {delete: [{c: "c1"}]}
         |  }
         |  create:{p:"Should not matter"}
         |  ){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
      project
    )

    res.toString should be("""{"data":{"upsertParent":{"childrenOpt":[]}}}""")

    server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[]}}""")

  }

  "a one to many relation" should "be deletable by id through a nested mutation" ignore {
    val schema = s"""model Comment{
                            id   String  @id @default(cuid())
                            text String?
                            todo Todo?
                        }

                        model Todo{
                            id       String    @id @default(cuid())
                            text     String?
                            comments Comment[] $relationInlineDirective
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
    database.setup(project)

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
         |  upsertTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    update:{
         |      comments: {
         |        delete: [{id: "$comment1Id"}, {id: "$comment2Id"}]
         |      }
         |    }
         |    create:{text: "Should not matter"}
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.upsertTodo.comments").toString, """[]""")

    val query = server.query("""{ comments { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"comments":[]}}""")

  }

  "a one to many relation" should "be deletable by any unique argument through a nested mutation" ignore {

    val schema = s"""model Comment{
                            id     String @id @default(cuid())
                            text   String
                            alias  String @unique
                            todo   Todo?
                        }

                        model Todo{
                            id       String    @id @default(cuid())
                            text     String?
                            comments Comment[] $relationInlineDirective
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
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
         |  upsertTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    update:{
         |      comments: {
         |        delete: [{alias: "alias1"}, {alias: "alias2"}]
         |      }
         |    }
         |    create:{text:"Should not matter"}
         |  ){
         |    comments {
         |      text
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.upsertTodo.comments").toString, """[]""")

    val query = server.query("""{ comments { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"comments":[]}}""")

  }

  "a many to one relation" should "be deletable by id through a nested mutation" ignore {
    val schema = s"""model Comment{
                            id   String  @id @default(cuid())
                            text String?
                            todo Todo?
                        }

                        model Todo{
                            id       String    @id @default(cuid())
                            text     String?
                            comments Comment[] $relationInlineDirective
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
    database.setup(project)

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
         |  upsertComment(
         |    where: {
         |      id: "$commentId"
         |    }
         |    update: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |    create:{text:"Should not matter"}
         |
         |  ){
         |    todo {
         |      id
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsJsValue("data.upsertComment").toString, """{"todo":null}""")

    val query = server.query("""{ todoes { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")
  }

  "one2one relation both exist and are connected" should "be deletable by id through a nested mutation" ignore {
    val schema = """model Note{
                            id    String @id @default(cuid())
                            text  String?
                            todo  Todo?
                        }

                        model Todo{
                            id    String @id @default(cuid())
                            title String
                            note  Note?  @relation(references: [id])
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
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
         |  upsertNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    update: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |    create:{text:"Should not matter"}
         |
         |  ){
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )
    mustBeEqual(result.pathAsJsValue("data.upsertNote").toString, """{"todo":null}""")

    val query = server.query("""{ todoes { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")
  }

  "one2one relation both exist and are connected" should "be deletable by unique field through a nested mutation" ignore {
    val schema = """model Note{
                            id   String  @id @default(cuid())
                            text String? @unique
                            todo Todo?
                        }

                        model Todo{
                            id    String @id @default(cuid())
                            title String @unique
                            note  Note?  @relation(references: [id])
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
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
         |  upsertNote(
         |    where: {
         |      text: "FirstUnique"
         |    }
         |    update: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |    create:{text:"Should not matter"}
         |  ){
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.upsertNote").toString, """{"todo":null}""")

    val query = server.query("""{ todoes { id }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")

    val query2 = server.query("""{ notes { text }}""", project)
    mustBeEqual(query2.toString, """{"data":{"notes":[{"text":"FirstUnique"}]}}""")
  }

  "a one to one relation" should "not do a nested delete by id if the nested node does not exist" ignore {
    val schema = """model Note{
                            id   String  @id @default(cuid())
                            text String?
                            todo Todo?
                        }

                        model Todo{
                            id    String @id @default(cuid())
                            title String
                            note  Note?  @relation(references: [id])
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
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
      s"""mutation {
         |  upsertNote(
         |    where: {id: "$noteId"}
         |    update: {
         |      todo: {
         |        delete: true
         |      }
         |    }
         |    create:{text:"Should not matter"}
         |  ){
         |    todo {
         |      title
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 3041,
      errorContains =
        s"The relation NoteToTodo has no node for the model Note with the value '$noteId' for the field 'id' connected to a node for the model Todo on your mutation path."
    )

    val query = server.query("""{ todoes { title }}""", project)
    mustBeEqual(query.toString, """{"data":{"todoes":[]}}""")

    val query2 = server.query("""{ notes { text }}""", project)
    mustBeEqual(query2.toString, """{"data":{"notes":[{"text":"Note"}]}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String   @id @default(cuid())
                                             |  nameTop String   @unique
                                             |  middles Middle[] $relationInlineDirective
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String @id @default(cuid())
                                             |  nameMiddle String @unique
                                             |  tops       Top[]
                                             |  bottoms    Bottom[] $relationInlineDirective
                                             |}
                                             |
                                             |model Bottom {
                                             |  id         String @id @default(cuid())
                                             |  nameBottom String @unique
                                             |  middles    Middle
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
      s"""mutation b {
         |  updateTop(
         |    where: {nameTop: "the top"},
         |    data: {
         |      nameTop: "updated top",
         |      middles: {
         |        upsert: [{
         |              where: {nameMiddle: "the middle"},
         |              update:{  nameMiddle: "updated middle"
         |                      bottoms: {delete: [{nameBottom: "the bottom"}]}}
         |              create:{nameMiddle:"Should not matter"}
         |         }]
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middles (orderBy: id_ASC){
         |      nameMiddle
         |      bottoms (orderBy: id_ASC){
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
                                             |  middles Middle[] $relationInlineDirective
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String   @id @default(cuid())
                                             |  nameMiddle String   @unique
                                             |  bottoms    Bottom[] $relationInlineDirective
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
      s"""mutation b {
         |  updateTop(
         |    where: {nameTop: "the top"},
         |    data: {
         |      nameTop: "updated top",
         |      middles: {
         |        upsert: [{
         |              where:{nameMiddle: "the middle"},
         |              update:{nameMiddle: "updated middle"
         |                      bottoms: {delete: [{nameBottom: "the bottom"}]}}
         |              create:{nameMiddle:"Should not matter"}
         |              }]
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middles (orderBy: id_ASC){
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
                                             |  middles Middle[] $relationInlineDirective
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
                                             |  id         String @id @default(cuid())
                                             |  nameBottom String @unique
                                             |  middle     Middle
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
      s"""mutation b {
         |  updateTop(
         |    where: {nameTop: "the top"},
         |    data: {
         |      nameTop: "updated top",
         |      middles: {
         |        upsert: [{
         |              where: {nameMiddle: "the middle"},
         |              update:{nameMiddle: "updated middle"
         |                      bottom: {delete: true}}
         |              create:{nameMiddle:"Should not matter"}
         |              }]
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middles (orderBy: id_ASC) {
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
                                             |  id          String  @id @default(cuid())
                                             |  nameBottom  String  @unique
                                             |  below       Below[] $relationInlineDirective
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
      s"""mutation b {
         |  updateTop(
         |    where: {nameTop: "the top"},
         |    data: {
         |      nameTop: "updated top",
         |      middle: {
         |        update: {
         |               nameMiddle: "updated middle"
         |               bottom: {
         |                upsert: {
         |                  update:{
         |                    nameBottom: "updated bottom"
         |                    below: { delete: {nameBelow: "below"}}}
         |                create:{nameBottom:"Should not matter"}
         |          }
         |         }
         |       }
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middle {
         |      nameMiddle
         |      bottom {
         |        nameBottom
         |        below{
         |           nameBelow
         |        }
         |
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom","below":[{"nameBelow":"second below"}]}}}}}""")
  }
}
