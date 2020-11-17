package writes.nestedMutations.alreadyConverted

import java.util.UUID

import org.scalatest.{Matchers, WordSpecLike}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedCreateMutationInsideCreateSpec extends WordSpecLike with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "a P1! to C1 relation should work" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val child1Id = t.child.where(
        server
          .query(
            s"""mutation {
          |  createParent(data: {
          |    p: "p1"
          |    p_1: "p_1"
          |    p_2: "p_2"
          |    childReq: {
          |      create: {
          |        c: "c1"
          |        c_1: "c_1"
          |        c_2: "c_2"
          |      }
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
    }
  }

  "a P1 to C1 relation should work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childOpt: {
            |      create: {
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      }
            |    }
            |  }){
            |   childOpt{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

    }
  }

  "a PM to C1! should work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childrenOpt: {
            |      create: [{
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      },{
            |        c:"c2"
            |        c_1: "c2_1"
            |        c_2: "c2_2"
            |      }]
            |    }
            |  }){
            |   childrenOpt{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

    }
  }

  "a P1 to C1! relation  should work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childOpt: {
            |      create: {
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      }
            |    }
            |  }){
            |   childOpt{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

    }
  }

  "a PM to C1 relation should work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childrenOpt: {
            |      create: [{
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      },{
            |        c:"c2"
            |        c_1: "c2_1"
            |        c_2: "c2_2"
            |      }]
            |    }
            |  }){
            |   childrenOpt{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

    }
  }

  "a P1! to CM  relation  should work" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childReq: {
            |      create: {
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      }
            |    }
            |  }){
            |   childReq{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childReq":{"c":"c1"}}}}""")

    }
  }

  "a P1 to CM relation should work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childOpt: {
            |      create: {
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      }
            |    }
            |  }){
            |   childOpt{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childOpt":{"c":"c1"}}}}""")

      // make sure it is traversable in the opposite direction as well
      val queryResult = server.query(
        """
          |{
          |  children {
          |    parentsOpt {
          |      p
          |    }
          |  }
          |}
        """,
        project
      )

      queryResult.toString should be("""{"data":{"children":[{"parentsOpt":[{"p":"p1"}]}]}}""")

    }
  }

  "a PM to CM relation should work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          """mutation {
            |  createParent(data: {
            |    p: "p1"
            |    p_1: "p_1"
            |    p_2: "p_2"
            |    childrenOpt: {
            |      create: [{
            |        c: "c1"
            |        c_1: "c_1"
            |        c_2: "c_2"
            |      },{
            |        c:"c2"
            |        c_1: "c2_1"
            |        c_2: "c2_2"
            |      }]
            |    }
            |  }){
            |   childrenOpt{
            |     c
            |   }
            |  }
            |}""",
          project
        )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

    }
  }

  // todo other test

  "a one to many relation should be creatable through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo{
        |   id        String    @id @default(cuid())
        |   comments  Comment[] $relationInlineAttribute
        |}
        |
        |model Comment{
        |   id    String @id @default(cuid())
        |   text  String
        |   todo  Todo?
        |}"""
    }

    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createTodo(data:{
        |    comments: {
        |      create: [{text: "comment1"}, {text: "comment2"}]
        |    }
        |  }){
        |    id
        |    comments {
        |      text
        |    }
        |  }
        |}
      """,
      project
    )
    mustBeEqual(result.pathAsJsValue("data.createTodo.comments").toString, """[{"text":"comment1"},{"text":"comment2"}]""")
  }

  "a many to one relation should be creatable through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      """model Todo{
        |   id       String   @id @default(cuid())
        |   title    String
        |   comments Comment? @relation(references: [id])
        |}
        |
        |model Comment{
        |   id   String @id @default(cuid())
        |   text String
        |   todo Todo?
        |}"""
    }

    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createComment(data: {
        |    text: "comment1"
        |    todo: {
        |      create: {title: "todo1"}
        |    }
        |  }){
        |    id
        |    todo {
        |      title
        |    }
        |  }
        |}
      """,
      project
    )
    mustBeEqual(result.pathAsString("data.createComment.todo.title"), "todo1")
  }

  "a many to many relation should be creatable through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo{
        |   id     String @id @default(cuid())
        |   title  String
        |   tags   Tag[]
        |}
        |
        |model Tag{
        |   id    String @id @default(cuid())
        |   name  String
        |   todos Todo[]
        |}"""
    }

    database.setup(project)

    val result = server
      .query(
        """
        |mutation {
        |  createTodo(data:{
        |    title: "todo1"
        |    tags: {
        |      create: [{name: "tag1"}, {name: "tag2"}]
        |    }
        |  }){
        |    id
        |    tags {
        |      name
        |    }
        |  }
        |}
      """,
        project
      )

    mustBeEqual(result.pathAsJsValue("data.createTodo.tags").toString, """[{"name":"tag1"},{"name":"tag2"}]""")

    val result2 = server.query(
      """
        |mutation {
        |  createTag(data:{
        |    name: "tag1"
        |    todos: {
        |      create: [{title: "todo1"}, {title: "todo2"}]
        |    }
        |  }){
        |    id
        |    todos {
        |      title
        |    }
        |  }
        |}
      """,
      project
    )
    mustBeEqual(result2.pathAsJsValue("data.createTag.todos").toString, """[{"title":"todo1"},{"title":"todo2"}]""")
  }

  "A nested create on a one to one relation should correctly assign violations to offending model and not partially execute first direction" ignore {
    val project = SchemaDsl.fromStringV11() {
      """model User{
        |   id     String  @id @default(cuid())
        |   name   String
        |   unique String? @unique
        |   post   Post?   @relation(references: [id])
        |}
        |
        |model Post{
        |   id         String @id @default(cuid())
        |   title      String
        |   uniquePost String? @unique
        |   user       User?
        |}"""
    }

    database.setup(project)

    server.query(
      """mutation{
        |  createUser(data:{
        |    name: "Paul"
        |    unique: "uniqueUser"
        |    post: {create:{title: "test"    uniquePost: "uniquePost"}
        |    }
        |  })
        |    {id}
        |  }
      """,
      project
    )

    server.query("query{users{id}}", project).pathAsSeq("data.users").length should be(1)
    server.query("query{posts{id}}", project).pathAsSeq("data.posts").length should be(1)

    val errorTarget = () match {
      case _ if connectorTag == ConnectorTag.MySqlConnectorTag => "constraint: `unique`"
      case _                                                   => "fields: (`unique`)"
    }

    server.queryThatMustFail(
      """mutation{
        |  createUser(data:{
        |    name: "Paul2"
        |    unique: "uniqueUser"
        |    post: {create:{title: "test2"    uniquePost: "uniquePost2"}
        |    }
        |  })
        |    {id}
        |  }
      """,
      project,
      errorCode = 2002,
      errorContains = s"Unique constraint failed on the $errorTarget"
    )

    server.query("query{users{id}}", project).pathAsSeq("data.users").length should be(1)
    server.query("query{posts{id}}", project).pathAsSeq("data.posts").length should be(1)
  }

  "A nested create on a one to one relation should correctly assign violations to offending model and not partially execute second direction" ignore {
    val project = SchemaDsl.fromStringV11() {
      """model User{
        |   id      String  @id @default(cuid())
        |   name    String
        |   unique  String? @unique
        |   post    Post?   @relation(references: [id])
        |}
        |
        |model Post{
        |   id         String @id @default(cuid())
        |   title      String
        |   uniquePost String? @unique
        |   user       User?
        |}"""
    }

    database.setup(project)

    server.query(
      """mutation{
        |  createUser(data:{
        |    name: "Paul"
        |    unique: "uniqueUser"
        |    post: {create:{title: "test"    uniquePost: "uniquePost"}
        |    }
        |  })
        |    {id}
        |  }
      """,
      project
    )

    server.query("query{users{id}}", project).pathAsSeq("data.users").length should be(1)
    server.query("query{posts{id}}", project).pathAsSeq("data.posts").length should be(1)

    val errorTarget = () match {
      case _ if connectorTag == ConnectorTag.MySqlConnectorTag => "constraint: `uniquePost`"
      case _                                                   => "fields: (`uniquePost`)"
    }

    server.queryThatMustFail(
      """mutation{
        |  createUser(data:{
        |    name: "Paul2"
        |    unique: "uniqueUser2"
        |    post: {create:{title: "test2"    uniquePost: "uniquePost"}
        |    }
        |  })
        |    {id}
        |  }
      """,
      project,
      errorCode = 2002,
      errorContains = s"Unique constraint failed on the $errorTarget"
    )

    ifConnectorIsNotMongo(server.query("query{users{id}}", project).pathAsSeq("data.users").length should be(1))
    server.query("query{posts{id}}", project).pathAsSeq("data.posts").length should be(1)
  }

  "a deeply nested mutation should execute all levels of the mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model List{
        |   id    String @id @default(cuid())
        |   name  String
        |   todos Todo[] $relationInlineAttribute
        |}
        |
        |model Todo{
        |   id     String @id @default(cuid())
        |   title  String
        |   list   List?
        |   tag    Tag?   @relation(references: [id])
        |}
        |
        |model Tag{
        |   id    String @id @default(cuid())
        |   name  String
        |   todo  Todo?
        |}"""
    }

    database.setup(project)

    val mutation =
      """
        |mutation  {
        |  createList(data: {
        |    name: "the list",
        |    todos: {
        |      create: [
        |        {
        |          title: "the todo"
        |          tag: {
        |            create: {
        |              name: "the tag"
        |            }
        |          }
        |        }
        |      ]
        |    }
        |  }) {
        |    name
        |    todos {
        |      title
        |      tag {
        |        name
        |      }
        |    }
        |  }
        |}
      """

    val result = server.query(mutation, project)
    result.pathAsString("data.createList.name") should equal("the list")
    result.pathAsString("data.createList.todos.[0].title") should equal("the todo")
    result.pathAsString("data.createList.todos.[0].tag.name") should equal("the tag")
  }

  "a required one2one relation should be creatable through a nested create mutation" ignore {

    val project = SchemaDsl.fromStringV11() {
      """model Comment{
        |   id           String  @id @default(cuid())
        |   reqOnComment String
        |   optOnComment String?
        |   todo         Todo    @relation(references: [id])
        |}
        |
        |model Todo{
        |   id        String @id @default(cuid())
        |   reqOnTodo String
        |   optOnTodo String?
        |   comment   Comment
        |}"""
    }

    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createComment(data: {
        |    reqOnComment: "comment1"
        |    todo: {
        |      create: {reqOnTodo: "todo1"}
        |    }
        |  }){
        |    id
        |    todo{reqOnTodo}
        |  }
        |}
      """,
      project
    )
    mustBeEqual(result.pathAsString("data.createComment.todo.reqOnTodo"), "todo1")

    server.queryThatMustFail(
      """
        |mutation {
        |  createComment(data: {
        |    reqOnComment: "comment1"
        |    todo: {}
        |  }){
        |    id
        |    todo {
        |      reqOnTodo
        |    }
        |  }
        |}
      """,
      project,
      errorCode = 2011,
      errorContains = "Null constraint violation on the fields: (`todo`)"
    )
  }

  "a required one2one relation should be creatable through a nested connected mutation" ignore {

    val project = SchemaDsl.fromStringV11() {
      """model Comment{
        |   id           String @id @default(cuid())
        |   reqOnComment String
        |   optOnComment String?
        |   todo         Todo    @relation(references: [id])
        |}
        |
        |model Todo{
        |   id        String @id @default(cuid())
        |   reqOnTodo String
        |   optOnTodo String?
        |   comment   Comment?
        |}"""
    }

    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createComment(data: {
        |    reqOnComment: "comment1"
        |    todo: {
        |      create: {reqOnTodo: "todo1"}
        |    }
        |  }){
        |    id
        |    todo{
        |       reqOnTodo
        |    }
        |  }
        |}
      """,
      project
    )
    mustBeEqual(result.pathAsString("data.createComment.todo.reqOnTodo"), "todo1")

    server.query("{ todoes { id } }", project).pathAsSeq("data.todoes").size should be(1)
    server.query("{ comments { id } }", project).pathAsSeq("data.comments").size should be(1)

    // TODO: originally the argument had empty arguments for todo: {} in there. Our schema could not handle that yet.
    server.queryThatMustFail(
      """
        |mutation {
        |  createComment(data: {
        |    reqOnComment: "comment2"
        |  }){
        |    id
        |    todo {
        |      reqOnTodo
        |    }
        |  }
        |}
      """,
      project,
      errorCode = 2012,
      errorContains = "Missing a required value at `Mutation.createComment.data.CommentCreateInput.todo`"
    )

    server.query("{ todoes { id } }", project).pathAsSeq("data.todoes").size should be(1)
    server.query("{ comments { id } }", project).pathAsSeq("data.comments").size should be(1)

    val todoId = server
      .query(
        """
        |mutation {
        |  createTodo(data: {
        |       reqOnTodo: "todo2"
        |  })
        |  {
        |    id
        |  }
        |}
      """,
        project
      )
      .pathAsString("data.createTodo.id")

    server.query("{ todoes { id } }", project).pathAsSeq("data.todoes").size should be(2)
    server.query("{ comments { id } }", project).pathAsSeq("data.comments").size should be(1)

    server.query(
      s"""
        |mutation {
        |  createComment(data: {
        |    reqOnComment: "comment3"
        |    todo: {
        |      connect: {id: "$todoId"}
        |    }
        |  }){
        |    id
        |    todo{
        |       reqOnTodo
        |    }
        |  }
        |}
      """,
      project
    )

    server.query("{ todoes { id } }", project).pathAsSeq("data.todoes").size should be(2)
    server.query("{ comments { id } }", project).pathAsSeq("data.comments").size should be(2)

  }

  "creating a nested item with an id of model UUID should work" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""
         |model List {
         |  id     String @id @default(cuid())
         |  todos  Todo[]
         |}
         |
         |model Todo {
         |  id    String @id @default(uuid())
         |  title String
         |}
       """
    }
    database.setup(project)

    val result = server.query(
      """
        |mutation {
        |  createList(data: {
        |    todos: {
        |      create: [ {title: "the todo"} ]
        |    }
        |  }){
        |    todos {
        |      id
        |      title
        |    }
        |  }
        |}
      """,
      project
    )

    result.pathAsString("data.createList.todos.[0].title") should equal("the todo")
    val theUuid = result.pathAsString("data.createList.todos.[0].id")
    UUID.fromString(theUuid) // should now blow up
  }

  "Backrelation bug should be fixed" ignore {

    val project = SchemaDsl.fromStringV11() {
      s"""
        |model User {
        |  id          String           @id @default(cuid())
        |  nick        String           @unique
        |  memberships ListMembership[]
        |}
        |
        |model List {
        |  id          String   @id @default(cuid())
        |  createdAt   DateTime @default(now())
        |  updatedAt   DateTime @updatedAt
        |  name        String
        |  memberships ListMembership[]
        |}
        |
        |model ListMembership {
        |  id   String @id @default(cuid())
        |  user User   @relation(references: [id])
        |  list List   @relation(references: [id])
        |}"""
    }

    database.setup(project)

    val create = server.query(
      s"""mutation createUser {
                  createUser(data: {
                    nick: "marcus"
                    memberships: {
                      create: [
                        {
                          list: {
                            create: {
                              name: "Personal Inbox"
                            }
                          }
                        }
                      ]
                    }
                  }){
                    nick
                  }
                }""",
      project
    )

    create.toString should be("""{"data":{"createUser":{"nick":"marcus"}}}""")

    val result = server.query(
      s"""query users {
                  users{
                    nick
                    memberships {
                      list {
                        name
                      }
                    }
                  }
                }""",
      project
    )

    result.toString should be("""{"data":{"users":[{"nick":"marcus","memberships":[{"list":{"name":"Personal Inbox"}}]}]}}""")
  }
}
