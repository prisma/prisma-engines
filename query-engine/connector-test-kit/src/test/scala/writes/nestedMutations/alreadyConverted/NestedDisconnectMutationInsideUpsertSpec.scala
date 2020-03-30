package writes.nestedMutations.alreadyConverted

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedDisconnectMutationInsideUpsertSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "a P1 to C1 relation " should "be disconnectable through a nested mutation by id" in {
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
          |      create: {
          |        c: "c1"
          |        c_1: "c_1"
          |        c_2: "c_2"
          |      }
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
         |    where: $parentId
         |    update:{
         |      p: "p2"
         |      childOpt: {disconnect: true}
         |    }
         |    create:{p: "Should not Matter"}
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

  "a P1 to C1  relation with the child and the parent without a relation" should "not be disconnectable through a nested mutation by id" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
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
         |  where: $parent1Id
         |  update:{
         |    p: "p2"
         |    childOpt: {disconnect: true}
         |  }
         |  create: {
         |    p:"Should not Matter"
         |  }
         |  ){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2017,
        errorContains = """The records for relation `ChildToParent` between the `Parent` and `Child` models are not connected.""",
      )

    }
  }

  "a PM to C1!  relation with the child already in a relation" should "not be disconnectable through a nested mutation by unique" in {
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

      server.queryThatMustFail(
        s"""mutation {
         |  upsertParent(
         |    where: $parentIdentifier
         |    update:{
         |    childrenOpt: {disconnect: {c: "c1"}}
         |    }
         |    create: {p: "Should not Matter"}
         |  ){
         |    childrenOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2014,
        errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."""
      )

    }
  }

  "a P1 to C1!  relation with the child and the parent already in a relation" should "should error in a nested mutation by unique" in {
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
      val parentIdentifier = t.parent.where(parentResult, "data.createParent")

      server.queryThatMustFail(
        s"""mutation {
         |  upsertParent(
         |  where: $parentIdentifier
         |  update:{
         |    childOpt: {disconnect: true}
         |  }
         |  create: {p: "Should not Matter"}
         |  ){
         |    childOpt {
         |      c
         |    }
         |  }
         |}
      """,
        project,
        errorCode = 2014,
        errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."""
      )

    }
  }

  "a PM to C1  relation with the child already in a relation" should "be disconnectable through a nested mutation by unique" in {
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
         |    childrenOpt: {disconnect: [{c: "c2"}]}
         |  }
         |  create: {p: "Should not Matter"}
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

  "a P1 to CM  relation with the child already in a relation" should "be disconnectable through a nested mutation by unique" in {
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
        |      create: {
        |        c: "c1"
        |        c_1: "c_1"
        |        c_2: "c_2"
        |      }
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
         |    update:{
         |    childOpt: {disconnect: true}
         |  }
         |    create: {p: "Should not Matter"}
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

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be("""{"data":{"children":[{"c":"c1","parentsOpt":[]}]}}""")

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
         |  upsertParent(
         |  where: $parentIdentifier
         |  update:{
         |    childrenOpt: {disconnect: [{c: "c1"}]}
         |  }
         |  create: {p: "Should not Matter"}
         |  ){
         |    childrenOpt{
         |      c
         |    }
         |  }
         |}
      """,
        project
      )

      res.toString should be("""{"data":{"upsertParent":{"childrenOpt":[{"c":"c2"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[]},{"c":"c2","parentsOpt":[{"p":"p1"}]}]}}""")

    }
  }

  // OTHER DATAMODELS

  "a one to many relation" should "be disconnectable by id through a nested mutation" ignore {
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
         |        disconnect: [{id: "$comment1Id"}, {id: "$comment2Id"}]
         |      }
         |    }
         |    create: {comments: {
         |        create: [{text: "comment3"}, {text: "comment4"}]
         |      }}
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
  }

  "a one to many relation" should "be disconnectable by any unique argument through a nested mutation" ignore {
    val schema = s"""model Comment{
                            id    String  @id @default(cuid())
                            text  String?
                            alias String  @unique
                            todo  Todo?
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
         |        disconnect: [{alias: "alias1"}, {alias: "alias2"}]
         |      }
         |    }
         |    create: {
         |      comments: {
         |        create: [{text: "comment3", alias: "alias4"}, {text: "comment4",alias: "alias5"}]
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

    mustBeEqual(result.pathAsJsValue("data.upsertTodo.comments").toString, """[]""")
  }

  "a many to one relation" should "be disconnectable by id through a nested mutation" ignore {
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
         |      todo: {disconnect: true}
         |    }
         |    create: {todo: {
         |        create: {text: "todo4"}
         |      }}
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
  }

  "a one to one relation" should "be disconnectable by id through a nested mutation" ignore {
    val schema = """model Note{
                            id   String  @id @default(cuid())
                            text String?
                            todo Todo?   @relation(references: [id])
                        }

                        model Todo{
                            id    String @id @default(cuid())
                            title String
                            note  Note?
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
         |      todo: {   disconnect: true}
         |    }
         |    create:{text: "Should not matter", todo: {create:{title: "Also doesnt matter"}}}
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
  }

  "a one to many relation" should "be disconnectable by unique through a nested mutation" ignore {
    val schema = s"""model Comment{
                            id   String  @id @default(cuid())
                            text String? @unique
                            todo Todo?
                        }

                        model Todo{
                            id       String    @id @default(cuid())
                            title    String?   @unique
                            comments Comment[] $relationInlineDirective
                        }"""

    val project = SchemaDsl.fromStringV11() { schema }
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

    val result2 = server.query(
      s"""mutation {
         |  upsertTodo(
         |    where: {
         |      title: "todo"
         |    }
         |    update:{
         |      comments: {
         |        disconnect: [{text: "comment2"}]
         |      }
         |    }
         |    create:{
         |      comments: {
         |        create: [{text: "comment3"}, {text: "comment4"}]
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

    mustBeEqual(result2.pathAsJsValue("data.upsertTodo.comments").toString, """[{"text":"comment1"}]""")
  }

  "A PM CM self relation" should "be disconnectable by unique through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() { s"""|
                                              |model User {
                                              |  id            String  @id @default(cuid())
                                              |  banned        Boolean @default(value: false)
                                              |  username      String  @unique
                                              |  password      String
                                              |  salt          String
                                              |  followers     User[]  @relation(name: "UserFollowers" $listInlineArgument)
                                              |  followersBack User[]  @relation(name: "UserFollowers")
                                              |  follows       User[]  @relation(name: "UserFollows" $listInlineArgument)
                                              |  followsBack   User[]  @relation(name: "UserFollows")
                                              |}""" }
    database.setup(project)

    server.query("""mutation { createUser(data: {username: "Paul", password: "1234", salt: "so salty"}){ id } }""", project)
    server.query("""mutation { createUser(data: {username: "Peter", password: "abcd", salt: "so salty"}){ id } }""", project)

    val result = server.query(
      s"""mutation {
         |  updateUser(
         |    where: {
         |      username: "Paul"
         |    }
         |    data:{
         |      follows: {
         |        connect: [{username: "Peter"}]
         |      }
         |    }
         |  ){
         |    username
         |    follows {
         |      username
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateUser.follows").toString, """[{"username":"Peter"}]""")

    val result2 = server.query(
      s"""mutation {
         |  upsertUser(
         |    where: {
         |      username: "Paul"
         |    }
         |    update:{
         |      follows: {
         |        disconnect: [{username: "Peter"}]
         |      }
         |    }
         |    create:{
         |      username: "Paul3",
         |      password: "1234",
         |      salt: "so salty"
         |
         |      follows: {
         |        create: [{username: "Paul2", password: "1234", salt: "so salty"}]
         |      }
         |    }
         |  ){
         |    username
         |    follows {
         |      username
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result2.pathAsJsValue("data.upsertUser.follows").toString, """[]""")
  }

  "A PM CM self relation" should "should throw a correct error for disconnect on invalid unique" ignore {
    val project = SchemaDsl.fromStringV11() { s"""|model User {
                                              |  id            String  @id @default(cuid())
                                              |  banned        Boolean @default(value: false)
                                              |  username      String  @unique
                                              |  password      String
                                              |  salt          String
                                              |  followers     User[] @relation(name: "UserFollowers" $listInlineArgument)
                                              |  followersBack User[] @relation(name: "UserFollowers")
                                              |  follows       User[] @relation(name: "UserFollows" $listInlineArgument)
                                              |  followsBack   User[] @relation(name: "UserFollows")
                                              |}""" }
    database.setup(project)

    server.query("""mutation { createUser(data: {username: "Paul", password: "1234", salt: "so salty"}){ id } }""", project)
    server.query("""mutation { createUser(data: {username: "Peter", password: "abcd", salt: "so salty"}){ id } }""", project)
    server.query("""mutation { createUser(data: {username: "Anton", password: "abcd3", salt: "so salty"}){ id } }""", project)

    val result = server.query(
      s"""mutation {
         |  updateUser(
         |    where: {
         |      username: "Paul"
         |    }
         |    data:{
         |      follows: {
         |        connect: [{username: "Peter"}]
         |      }
         |    }
         |  ){
         |    username
         |    follows {
         |      username
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsJsValue("data.updateUser.follows").toString, """[{"username":"Peter"}]""")

    server.queryThatMustFail(
      s"""mutation {
         |  upsertUser(
         |    where: {
         |      username: "Paul"
         |    }
         |    update:{
         |      follows: {
         |        disconnect: [{username: "Anton"}]
         |      }
         |    }
         |     create:{
         |      username: "Paul3",
         |      password: "1234",
         |      salt: "so salty"
         |      follows: {
         |        create: [{username: "Paul2", password: "1234", salt: "so salty"}]
         |      }
         |    }
         |  ){
         |    username
         |    follows {
         |      username
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 2017,
      errorContains = """Error occurred during query execution:\nInterpretationError(\"Error for binding \\'5\\': RecordsNotConnected { relation_name: \\\"UserFollows\\\", parent_name: \\\"User\\\", child_name: \\\"User\\\"""
    )
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String   @id @default(cuid())
                                             |  nameTop String   @unique
                                             |  middles Middle[] $relationInlineDirective
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String   @id @default(cuid())
                                             |  nameMiddle String   @unique
                                             |  tops       Top[]
                                             |  bottoms    Bottom[] $relationInlineDirective
                                             |}
                                             |
                                             |model Bottom {
                                             |  id          String   @id @default(cuid())
                                             |  nameBottom  String   @unique
                                             |  middles     Middle[]
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
         |              update:{nameMiddle: "updated middle"
         |                      bottoms: {disconnect: [{nameBottom: "the bottom"}]}
         |              },
         |              create:{nameMiddle: "Should not Matter"}
         |       }]
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

    server.query("query{bottoms(orderBy: id_ASC){nameBottom}}", project).toString should be(
      """{"data":{"bottoms":[{"nameBottom":"the bottom"},{"nameBottom":"the second bottom"},{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path and there are no backrelations" ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String   @id @default(cuid())
                                             |  nameTop String   @unique
                                             |  middles Middle[] $relationInlineDirective
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String @id @default(cuid())
                                             |  nameMiddle String @unique
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
         |              where: {nameMiddle: "the middle"},
         |              update:{nameMiddle: "updated middle"
         |                      bottoms: {disconnect: [{nameBottom: "the bottom"}]}},
         |              create:{nameMiddle: "Should not matter"}
         |       }]
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middles  (orderBy: id_ASC){
         |      nameMiddle
         |      bottoms  (orderBy: id_ASC){
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottoms":[{"nameBottom":"the second bottom"}]},{"nameMiddle":"the second middle","bottoms":[{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}]}}}""")

    server.query("query{bottoms (orderBy: id_ASC){nameBottom}}", project).toString should be(
      """{"data":{"bottoms":[{"nameBottom":"the bottom"},{"nameBottom":"the second bottom"},{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are model and node edges on the path " ignore {
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
      s"""mutation b {
         |  updateTop(
         |    where: {nameTop: "the top"},
         |    data: {
         |      nameTop: "updated top",
         |      middles: {
         |        upsert: [{
         |              where: {nameMiddle: "the middle"},
         |              update:{nameMiddle: "updated middle"
         |                      bottom: {disconnect: true}}
         |              create:{nameMiddle: "Should not matter"}
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

    server.query("query{bottoms{nameBottom}}", project).toString should be(
      """{"data":{"bottoms":[{"nameBottom":"the bottom"},{"nameBottom":"the second bottom"}]}}""")
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
                                             |  below      Below[] $relationInlineDirective
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
         |                  update:{nameBottom: "updated bottom", below: { disconnect: {nameBelow: "below"}}},
         |                  create:{nameBottom: "Should not matter"}
         |              }
         |          }
         |       }
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middle {
         |      nameMiddle
         |      bottom {
         |        nameBottom
         |        below (orderBy: id_ASC){
         |           nameBelow
         |        }
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom","below":[{"nameBelow":"second below"}]}}}}}""")

    server.query("query{belows (orderBy: id_ASC){nameBelow}}", project).toString should be(
      """{"data":{"belows":[{"nameBelow":"below"},{"nameBelow":"second below"}]}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only model edges on the path" ignore {
    val project = SchemaDsl.fromStringV11() { """model Top {
                                             |  id      String  @id @default(cuid())
                                             |  nameTop String  @unique
                                             |  middle  Middle? @relation(references: [id])
                                             |}
                                             |
                                             |model Middle {
                                             |  id         String  @id @default(cuid())
                                             |  nameMiddle String  @unique
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
         |mutation  {
         |  updateTop(
         |    where: {
         |      nameTop: "the top"
         |    }
         |    data: {
         |      nameTop: "updated top",
         |      middle: {
         |        upsert:{
         |          update:{nameMiddle: "updated middle", bottom: {disconnect: true}},
         |          create:{nameMiddle: "Should not Matter"}
         |        }
         |     }
         |   }
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

    server.query("query{bottoms{nameBottom}}", project).toString should be("""{"data":{"bottoms":[{"nameBottom":"the bottom"}]}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only model edges on the path and there are no backrelations" ignore {
    val project = SchemaDsl.fromStringV11() { """model Top {
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
         |mutation  {
         |  updateTop(
         |    where: {
         |      nameTop: "the top"
         |    }
         |    data: {
         |      nameTop: "updated top",
         |      middle: {
         |        upsert: {
         |              update:{nameMiddle: "updated middle",bottom: {disconnect: true}}
         |              create:{nameMiddle: "Should not matter"}
         |      }
         |     }
         |   }
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

    server.query("query{bottoms{nameBottom}}", project).toString should be("""{"data":{"bottoms":[{"nameBottom":"the bottom"}]}}""")
  }
}
