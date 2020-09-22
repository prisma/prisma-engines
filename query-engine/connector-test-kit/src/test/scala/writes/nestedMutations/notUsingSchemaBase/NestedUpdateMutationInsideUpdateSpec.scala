package writes.nestedMutations.notUsingSchemaBase

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedUpdateMutationInsideUpdateSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)

  "A P1! to C1! relation relation" should "work" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res =
        server
          .query(
            s"""mutation {
               |  createParent(data: {
               |    p: "p1", p_1: "p", p_2: "1",
               |    childReq: {
               |      create: {c: "c1", c_1: "c", c_2: "1"}
               |    }
               |  }){
               |  
               |    ${t.parent.selection}
               |    childReq{
               |       ${t.child.selection}
               |    }
               |  }
               |}""",
            project
          )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      val finalRes = server.query(
        s"""mutation {
           |  updateParent(
           |  where: $parentIdentifier
           |  data:{
           |    childReq: {
           |        update: { non_unique: { set: "updated" }}
           |      }
           |  }){
           |    childReq {
           |      non_unique
           |    }
           |  }
           |}""",
        project
      )

      finalRes.toString() should be("{\"data\":{\"updateParent\":{\"childReq\":{\"non_unique\":\"updated\"}}}}")

    }
  }

  "A P1 to CM relation relation" should "work" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res =
        server
          .query(
            s"""mutation {
               |  createParent(data: {
               |    p: "p1", p_1: "p", p_2: "1",
               |    childOpt: {
               |      create: {c: "c1", c_1: "c", c_2: "1"}
               |    }
               |  }){
               |  
               |    ${t.parent.selection}
               |    childOpt{
               |       ${t.child.selection}
               |    }
               |  }
               |}""",
            project
          )

      val parentIdentifier = t.parent.where(res, "data.createParent")

      val finalRes = server.query(
        s"""mutation {
           |  updateParent(
           |  where: $parentIdentifier
           |  data:{
           |    childOpt: {
           |      update: { non_unique: { set: "updated" }}
           |    }
           |  }){
           |    childOpt {
           |      non_unique
           |    }
           |  }
           |}""",
        project
      )

      finalRes.toString() should be("{\"data\":{\"updateParent\":{\"childOpt\":{\"non_unique\":\"updated\"}}}}")

    }
  }

  "A PM to C1 relation relation" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res =
        server
          .query(
            s"""mutation {
               |  createParent(data: {
               |    p: "p1", p_1: "p", p_2: "1",
               |    childrenOpt: {
               |      create: [{c: "c1", c_1: "c", c_2: "1"},{c: "c2", c_1: "c", c_2: "2"}]
               |    }
               |  }) {
               |    ${t.parent.selection}
               |    childrenOpt{
               |       ${t.child.selection}
               |    }
               |  }
               |}""",
            project
          )

      val parentIdentifier = t.parent.where(res, "data.createParent")
      val childIdentifier  = t.child.whereFirst(res, "data.createParent.childrenOpt")

      val finalRes = server.query(
        s"""mutation {
           |  updateParent(
           |  where: $parentIdentifier
           |  data:{
           |    childrenOpt: {
           |        update:  [
           |          { where: $childIdentifier, data: { non_unique: { set: "updated" } }}
           |        ]  
           |      }
           |  }){
           |    childrenOpt (orderBy: { c: asc } ){
           |      non_unique
           |    }
           |  }
           |}""",
        project
      )

      finalRes.toString() should be("{\"data\":{\"updateParent\":{\"childrenOpt\":[{\"non_unique\":\"updated\"},{\"non_unique\":null}]}}}")

    }
  }

  "A PM to CM relation relation" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res =
        server
          .query(
            s"""mutation {
               |  createParent(data: {
               |    p: "p1", p_1: "p", p_2: "1",
               |    childrenOpt: {
               |      create: [{c: "c1", c_1: "c", c_2: "1"},{c: "c2", c_1: "c", c_2: "2"}]
               |    }
               |  }){
               |  
               |    ${t.parent.selection}
               |    childrenOpt{
               |       ${t.child.selection}
               |    }
               |  }
               |}""",
            project
          )

      val parentIdentifier = t.parent.where(res, "data.createParent")
      val childIdentifier  = t.child.whereFirst(res, "data.createParent.childrenOpt")

      val finalRes = server.query(
        s"""mutation {
           |  updateParent(
           |  where: $parentIdentifier
           |  data:{
           |    childrenOpt: {
           |        update:  [
           |          {where: $childIdentifier, data: {non_unique: { set: "updated" }}}
           |        ]  
           |      }
           |  }){
           |    childrenOpt (orderBy: { c: asc } ){
           |      non_unique
           |    }
           |  }
           |}""",
        project
      )

      finalRes.toString() should be("{\"data\":{\"updateParent\":{\"childrenOpt\":[{\"non_unique\":\"updated\"},{\"non_unique\":null}]}}}")

    }
  }

  ///OLD

  "a one to many relation" should "be updateable by id through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo {
        | id       String    @id @default(cuid())
        | comments Comment[] $relationInlineDirective
        |}
        |
        |model Comment {
        | id   String  @id @default(cuid())
        | text String?
        | todo Todo
        |}
      """
    }
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
         |  updateTodo(
         |    where: {
         |      id: "$todoId"
         |    }
         |    data:{
         |      comments: {
         |        update: [
         |          {where: {id: "$comment1Id"}, data: {text: {set: "update comment1"}}},
         |          {where: {id: "$comment2Id"}, data: {text: {set: "update comment2"}}},
         |        ]
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

    mustBeEqual(result.pathAsString("data.updateTodo.comments.[0].text").toString, """update comment1""")
    mustBeEqual(result.pathAsString("data.updateTodo.comments.[1].text").toString, """update comment2""")
  }

  "a one to many relation" should "be updateable by any unique argument through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo {
        | id       String @id @default(cuid())
        | comments Comment[] $relationInlineDirective
        |}
        |
        |model Comment {
        | id    String @id @default(cuid())
        | alias String @unique
        | text  String?
        | todo  Todo
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
         |        update: [
         |          {where: {alias: "alias1"}, data: {text: {set: "update comment1"}}},
         |          {where: {alias: "alias2"}, data: {text: {set: "update comment2"}}}
         |        ]
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

    mustBeEqual(result.pathAsString("data.updateTodo.comments.[0].text").toString, """update comment1""")
    mustBeEqual(result.pathAsString("data.updateTodo.comments.[1].text").toString, """update comment2""")
  }

  "a many to many relation with an optional backrelation" should "be updateable by any unique argument through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model List {
        | id         String @id @default(cuid())
        | listUnique String @unique
        | todoes     Todo[] $relationInlineDirective
        |}
        |
        |model Todo {
        | id         String @id @default(cuid())
        | todoUnique String @unique
        |}
      """
    }
    database.setup(project)

    server.query(
      """mutation {
        |  createList(
        |    data: {
        |      listUnique : "list",
        |      todoes: {
        |        create: [{todoUnique: "todo"}]
        |      }
        |    }
        |  ){
        |    listUnique
        |    todoes { todoUnique }
        |  }
        |}""",
      project
    )
    val result = server.query(
      s"""mutation {
         |  updateList(
         |    where: {
         |      listUnique: "list"
         |    }
         |    data:{
         |      todoes: {
         |        update: [{where: {todoUnique: "todo"}, data: {todoUnique: {set: "new todo"}}}]
         |      }
         |    }
         |  ){
         |    listUnique
         |    todoes{
         |      todoUnique
         |    }
         |  }
         |}
      """,
      project
    )

    mustBeEqual(result.pathAsString("data.updateList.todoes.[0].todoUnique").toString, """new todo""")
  }

  "a many to one relation" should "be updateable by id through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo {
        | id       String    @id @default(cuid())
        | title    String?
        | comments Comment[] $relationInlineDirective
        |}
        |
        |model Comment {
        | id   String @id @default(cuid())
        | text String
        | todo Todo
        |}
      """
    }
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
         |  updateComment(
         |    where: {
         |      id: "$commentId"
         |    }
         |    data: {
         |      todo: {
         |        update: {title: {set: "updated title"}}
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
    mustBeEqual(result.pathAsJsValue("data.updateComment.todo").toString, """{"title":"updated title"}""")
  }

  "a one to one relation" should "be updateable by id through a nested mutation" ignore {
    val project = SchemaDsl.fromStringV11() {
      """model Todo {
        | id    String @id @default(cuid())
        | title String
        | note  Note?  @relation(references: [id])
        |}
        |
        |model Note {
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
         |        update: { title: { set: "updated title" }}
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
    mustBeEqual(result.pathAsJsValue("data.updateNote.todo").toString, """{"title":"updated title"}""")
  }

  //Transactionality
  "TRANSACTIONAL: a many to many relation" should "fail gracefully on wrong where and assign error correctly and not execute partially" taggedAs (IgnoreMongo) in {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo {
        | id    String @id @default(cuid())
        | title String
        | notes Note[] $relationInlineDirective
        |}
        |
        |model Note {
        | id     String  @id @default(cuid())
        | text   String?
        | todoes Todo[]
        |}
      """
    }
    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createNote(
        |    data: {
        |      text: "Some Text"
        |      todoes: {
        |        create: { title: "the title" }
        |      }
        |    }
        |  ){
        |    id
        |    todoes { id }
        |  }
        |}""",
      project
    )
    val noteId = createResult.pathAsString("data.createNote.id")
    val todoId = createResult.pathAsString("data.createNote.todoes.[0].id")

    server.queryThatMustFail(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    data: {
         |      text: { set: "Some Changed Text" }
         |      todoes: {
         |        update: {
         |          where: {id: "DOES NOT EXIST"},
         |          data:{ title: { set: "updated title" }}
         |        }
         |      }
         |    }
         |  ){
         |    text
         |  }
         |}
      """,
      project,
      errorCode = 2016,
      errorContains =
        """Query interpretation error. Error for binding '1': AssertionError(\"Expected a valid parent ID to be present for nested update to-one case.\")"""
      // No Node for the model Todo with value DOES NOT EXIST for id found.
    )

    server.query(s"""query{note(where:{id: "$noteId"}){text}}""", project, dataContains = """{"note":{"text":"Some Text"}}""")
    server.query(s"""query{todo(where:{id: "$todoId"}){title}}""", project, dataContains = """{"todo":{"title":"the title"}}""")
  }

  "NON-TRANSACTIONAL: a many to many relation" should "fail gracefully on wrong where and assign error correctly and not execute partially" in {
    val project = SchemaDsl.fromStringV11() {
      s"""model Todo {
        | id    String @id @default(cuid())
        | title String
        | notes Note[] $relationInlineDirective
        |}
        |
        |model Note {
        | id     String  @id @default(cuid())
        | text   String?
        | todoes Todo[]
        |}
      """
    }
    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createNote(
        |    data: {
        |      text: "Some Text"
        |      todoes: {
        |        create: { title: "the title" }
        |      }
        |    }
        |  ){
        |    id
        |    todoes { id }
        |  }
        |}""",
      project
    )
    val noteId = createResult.pathAsString("data.createNote.id")
    val todoId = createResult.pathAsString("data.createNote.todoes.[0].id")

    server.queryThatMustFail(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      id: "$noteId"
         |    }
         |    data: {
         |      text: { set: "Some Changed Text" }
         |      todoes: {
         |        update: {
         |          where: { id: "5beea4aa6183dd734b2dbd9b" },
         |          data:{ title: { set: "updated title" }}
         |        }
         |      }
         |    }
         |  ){
         |    text
         |  }
         |}
      """,
      project,
      errorCode = 2016,
      errorContains =
        "Query interpretation error. Error for binding '1': AssertionError(\\\"Expected a valid parent ID to be present for nested update to-one case."
    )
  }

  "a many to many relation" should "reject null in unique fields" in {
    val project = SchemaDsl.fromStringV11() {
      s"""model Note {
        | id    String  @id @default(cuid())
        | text  String? @unique
        | todos Todo[]  $relationInlineDirective
        |}
        |
        |model Todo {
        | id     String  @id @default(cuid())
        | title  String  @unique
        | unique String? @unique
        | notes  Note[]
        |}
      """
    }
    database.setup(project)

    val createResult = server.query(
      """mutation {
        |  createNote(
        |    data: {
        |      text: "Some Text"
        |      todos: {
        |       create: [{ title: "the title", unique: "test"}, { title: "the other title" }]
        |      }
        |    }
        |  ){
        |    id
        |    todos { id }
        |  }
        |}""",
      project
    )

    val result = server.queryThatMustFail(
      s"""
         |mutation {
         |  updateNote(
         |    where: {
         |      text: "Some Text"
         |    }
         |    data: {
         |      text: { set: "Some Changed Text" }
         |      todos: {
         |        update: {
         |          where: { unique: null },
         |          data: { title: { set: "updated title" }}
         |        }
         |      }
         |    }
         |  ){
         |    text
         |    todos {
         |      title
         |    }
         |  }
         |}
      """,
      project,
      errorCode = 2009, // 3040,
      errorContains =
        "`Mutation.updateNote.data.NoteUpdateInput.todos.TodoUpdateManyWithoutNotesInput.update.TodoUpdateWithWhereUniqueWithoutNotesInput.where.TodoWhereUniqueInput.unique`: A value is required but not set."
    )
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are only node edges on the path" in {
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
      s"""mutation b {
         |  updateTop(
         |    where: {nameTop: "the top"},
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middles: {
         |        update: [{
         |              where: { nameMiddle: "the middle" },
         |              data:{
         |                nameMiddle: { set: "updated middle" }
         |                bottoms: {
         |                  update: [{
         |                    where: { nameBottom: "the bottom" },
         |                    data:  { nameBottom: { set: "updated bottom" }}
         |                  }]
         |              }
         |            }
         |          }
         |        ]
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middles (orderBy: { id: asc }){
         |      nameMiddle
         |      bottoms (orderBy: { id: asc }){
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottoms":[{"nameBottom":"updated bottom"},{"nameBottom":"the second bottom"}]},{"nameMiddle":"the second middle","bottoms":[{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}]}}}""")
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
         |      nameTop: { set: "updated top" }
         |      middles: {
         |        update: [{
         |          where: { nameMiddle: "the middle" }
         |          data: {
         |            nameMiddle: { set: "updated middle" }
         |            bottoms: {
         |              update: [{
         |                where: { nameBottom: "the bottom" }
         |                data:  { nameBottom: { set: "updated bottom" }}
         |              }]
         |            }
         |          }
         |        }
         |      ]
         |     }
         |   }
         |  ) {
         |    nameTop
         |    middles  (orderBy: { id: asc }) {
         |      nameMiddle
         |      bottoms  (orderBy: { id: asc }){
         |        nameBottom
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottoms":[{"nameBottom":"updated bottom"},{"nameBottom":"the second bottom"}]},{"nameMiddle":"the second middle","bottoms":[{"nameBottom":"the third bottom"},{"nameBottom":"the fourth bottom"}]}]}}}""")
  }

  "a deeply nested mutation" should "execute all levels of the mutation if there are model and node edges on the path " ignore {
    val project = SchemaDsl.fromStringV11() { s"""model Top {
                                             |  id      String   @id @default(cuid())
                                             |  nameTop String   @unique
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
         |              bottom: { update: { nameBottom: { set: "updated bottom" } } }
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
         |
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middles":[{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom"}},{"nameMiddle":"the second middle","bottom":{"nameBottom":"the second bottom"}}]}}}""")
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
         |              below: {
         |                update: {
         |                  where: { nameBelow: "below" }
         |                  data: { nameBelow: { set: "updated below" } }
         |                }
         |              }
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
         |        below(orderBy: { id: asc }) {
         |          nameBelow
         |        }
         |      }
         |    }
         |  }
         |}
      """

    val result = server.query(updateMutation, project)

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom","below":[{"nameBelow":"updated below"},{"nameBelow":"second below"}]}}}}}""")
  }

  "a deeply nested mutation" should "fail if there are model and node edges on the path and back relations are missing and node edges follow model edges but the path is interrupted" ignore {
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
                                             |  id         String @id @default(cuid())
                                             |  nameBottom String @unique
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

    val createMutation2 =
      """
        |mutation a {
        |  createTop(data: {
        |    nameTop: "the second top",
        |    middle: {
        |      create:
        |        {
        |          nameMiddle: "the second middle"
        |          bottom: {
        |            create: { nameBottom: "the second bottom"
        |            below: {
        |            create: [{ nameBelow: "other below"}, { nameBelow: "second other below"}]}
        |        }}}
        |        }
        |  }) {id}
        |}
      """

    server.query(createMutation2, project)

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
         |              below: {
         |                update: {
         |                  where: { nameBelow: "other below" }
         |                  data: { nameBelow: { set: "updated below" } }
         |                }
         |              }
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

    server.queryThatMustFail(
      updateMutation,
      project,
      errorCode = 2016,
      errorContains =
        """Error occurred during query execution:\nInterpretationError(\"Error for binding \\'5\\': AssertionError(\\\"Expected a valid parent ID to be present for nested update to-one case.\\\""""
    )
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
                                             |  id         String @id @default(cuid())
                                             |  middle     Middle
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
         |          bottom: { update: { nameBottom: { set: "updated bottom" } } }
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

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom"}}}}}""")
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
         |mutation {
         |  updateTop(
         |    where: { nameTop: "the top" }
         |    data: {
         |      nameTop: { set: "updated top" }
         |      middle: {
         |        update: {
         |          nameMiddle: { set: "updated middle" }
         |          bottom: { update: { nameBottom: { set: "updated bottom" } } }
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

    result.toString should be(
      """{"data":{"updateTop":{"nameTop":"updated top","middle":{"nameMiddle":"updated middle","bottom":{"nameBottom":"updated bottom"}}}}}""")
  }

  "a deeply nested mutation" should "fail if there are only model edges on the path but there is no connected item to update at the end" ignore {
    val project = SchemaDsl.fromStringV11() { """model Top {
                                             |  id      String @id @default(cuid())
                                             |  nameTop String @unique
                                             |  middle  Middle @relation(references: [id])
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
        |      create:{ nameMiddle: "the middle"}
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
         |          bottom: { update: { nameBottom: { set: "updated bottom" } } }
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

    server.queryThatMustFail(
      updateMutation,
      project,
      errorCode = 2016,
      errorContains = """Query interpretation error"""
    )

  }
}
