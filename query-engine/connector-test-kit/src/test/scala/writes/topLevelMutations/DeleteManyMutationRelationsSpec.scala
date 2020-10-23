package writes.topLevelMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class DeleteManyMutationRelationsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  "a P0 to C1! relation " should "error when deleting the parent" in {
    val schema =
      """
        |model Parent{
        |    id String @id @default(cuid())
        |    p  String @unique
        |}
        |
        |model Child{
        |    id        String @id @default(cuid())
        |    c         String @unique
        |    parentId  String
        |    parentReq Parent @relation(fields: [parentId], references: [id])
        |}
      """.stripMargin

    val project = ProjectDsl.fromString { schema }
    database.setup(project)

    server
      .query(
        """mutation {
          |  createChild(data: {
          |    c: "c1"
          |    parentReq: {
          |      create: {p: "p1"}
          |    }
          |  }){
          |    id
          |  }
          |}""".stripMargin,
        project
      )

    server.queryThatMustFail(
      s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" }}
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 2014,
      errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models.""",
    )

  }

  "a P0 to C1! relation " should "error when deleting the parent with empty filter" in {
    val schema = """model Parent{
                            id String @id @default(cuid())
                            p  String @unique
                        }

                        model Child{
                            id        String @id @default(cuid())
                            c         String @unique
                            parentId  String

                            parentReq Parent @relation(fields: [parentId], references: [id])
                        }"""

    val project = ProjectDsl.fromString { schema }
    database.setup(project)

    server
      .query(
        """mutation {
          |  createChild(data: {
          |    c: "c1"
          |    parentReq: {
          |      create: {p: "p1"}
          |    }
          |  }){
          |    id
          |  }
          |}""".stripMargin,
        project
      )

    server.queryThatMustFail(
      s"""
         |mutation {
         |  deleteManyParents(
         |  where: {}
         |  ){
         |  count
         |  }
         |}
      """.stripMargin,
      project,
      errorCode = 2014,
      errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."""
    )

  }

  "a P1! to C1! relation " should "error when deleting the parent" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
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
          |    ${t.parent.selection}
          |    childReq{
          |       ${t.child.selection}
          |    }
          |  }
          |}""".stripMargin,
          project
        )
      val childId  = t.child.where(res, "data.createParent.childReq")
      val parentId = t.parent.where(res, "data.createParent")

      server.queryThatMustFail(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: $parentId
         |  ) {
         |    count
         |  }
         |}
      """.stripMargin,
        project,
        // TODO: errors are different depending on the relation setup
        errorCode = 0, // should be 2014,
        // errorContains =  """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models.""",
      )

    }
  }

  "a P1! to C1 relation" should "succeed when trying to delete the parent" in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentOpt, withoutParams = true).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1"
          |    p_1: "p_1"
          |    p_2: "p_2"
          |    childReq: {
          |      create: {
          |        c: "c1"
          |        c_1: "c_1",
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

      /*val parentId = t.parent.where(res, "data.createParent")*/

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: {
         |      p: { equals: "p1" }
         |    }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a P1 to C1  relation " should "succeed when trying to delete the parent" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt, withoutParams = true).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1"
          |    p_1: "1"
          |    p_2: "2"
          |    childOpt: {
          |      create: { c: "c1", c_1: "foo", c_2: "bar" }
          |    }
          |  }){
          |    p
          |    childOpt{
          |       c
          |    }
          |  }
          |}""".stripMargin,
          project
        )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: {
         |      p: { equals: "p1" }
         |    }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a P1 to C1  relation " should "succeed when trying to delete the parent if there are no children" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server
        .query(
          s"""mutation {
          |  createParent(data: {
          |    p: "p1" p_1: "lol" p_2: "zoop"
          |  }){
          |    ${t.parent.selection}
          |  }
          |}""".stripMargin,
          project
        )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )

    }
  }

  "a PM to C1!  relation " should "error when deleting the parent" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "p_1"
        |    p_2: "p_2"
        |    childrenOpt: {
        |      create: {c: "c1", c_1: "foo", c_2: "bar"}
        |    }
        |  }){
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""".stripMargin,
        project
      )

      server.queryThatMustFail(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project,
        errorCode = 2014,
        errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."""
      )

    }
  }

  "a PM to C1!  relation " should "succeed if no child exists that requires the parent" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1" p_1: "p1" p_2: "p2"
        |  }){
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )

    }

  }

  "a P1 to C1!  relation " should "error when trying to delete the parent" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "p_1"
        |    p_2: "p_2"
        |    childOpt: {
        |      create: {c: "c1", c_1: "foo", c_2: "bar"}
        |    }
        |  }){
        |    childOpt{
        |       c
        |    }
        |  }
        |}""".stripMargin,
        project
      )

      server.queryThatMustFail(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project,
        errorCode = 2014,
        errorContains = """The change you are trying to make would violate the required relation 'ChildToParent' between the `Child` and `Parent` models."""
      )

    }
  }

  "a P1 to C1!  relation " should "succeed when trying to delete the parent if there is no child" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentReq).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1" p_1: "p1" p_2: "p2"
        |  }){
        |    p
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )

    }
  }

  "a PM to C1 " should "succeed in deleting the parent" in {
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
          |    p_1: "1"
          |    p_2: "2"
          |    childrenOpt: {
          |      create: [{c: "c1", c_1: "foo", c_2: "bar"}, {c: "c2", c_1: "fqe", c_2: "asd"}]
          |    }
          |  }){
          |    childrenOpt{
          |       c
          |    }
          |  }
          |}""".stripMargin,
          project
        )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )

    }
  }

  "a PM to C1 " should "succeed in deleting the parent if there is no child" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentOpt).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server
        .query(
          """mutation {
          |  createParent(data: {
          |    p: "p1" p_1: "1" p_2: "2"
          |  }){
          |    p
          |  }
          |}""".stripMargin,
          project
        )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )

    }
  }

  "a P1! to CM  relation" should "should succeed in deleting the parent " in {
    schemaWithRelation(onParent = ChildReq, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "1"
        |    p_2: "2"
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

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |   where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a P1 to CM  relation " should " should succeed in deleting the parent" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "1"
        |    p_2: "2"
        |    childOpt: {
        |      create: {c: "c1", c_1: "foo", c_2: "bar"}
        |    }
        |  }){
        |    childOpt{
        |       c
        |    }
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |  where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a P1 to CM  relation " should " should succeed in deleting the parent if there is no child" in {
    schemaWithRelation(onParent = ChildOpt, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "1"
        |    p_2: "2"
        |  }){
        |    p
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a PM to CM  relation" should "succeed in deleting the parent" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "1"
        |    p_2: "2"
        |    childrenOpt: {
        |      create: [{c: "c1", c_1: "foo", c_2: "bar"},{c: "c2", c_1: "q23", c_2: "lk"}]
        |    }
        |  }){
        |    childrenOpt{
        |       c
        |    }
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |   where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a PM to CM  relation" should "succeed in deleting the parent if there is no child" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    p_1: "1"
        |    p_2: "2"
        |  }){
        |    p
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |   where: { p: { equals: "p1" } }
         |  ){
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }

  "a PM to CM  relation" should "delete the parent from other relations as well" in {
    val testDataModels = {
      // TODO: use new syntax for Mongo
      val dm1 = """model Parent{
                       id           String    @id @default(cuid())
                       p            String    @unique
                       childrenOpt  Child[]   @relation(references: [id])
                       stepChildOpt StepChild @relation(references: [id])
                   }

                   model Child{
                       id         String  @id @default(cuid())
                       c          String  @unique
                       parentsOpt Parent[]
                   }

                   model StepChild{
                        id        String  @id @default(cuid())
                        s         String  @unique
                        parentOpt Parent?
                   }"""

      val dm2 = """model Parent{
                       id           String     @id @default(cuid())
                       p            String     @unique
                       stepChildId  String?

                       childrenOpt  Child[]
                       stepChildOpt StepChild? @relation(fields: [stepChildId], references: [id])
                   }

                   model Child{
                       id         String @id @default(cuid())
                       c          String @unique

                       parentsOpt Parent[]
                   }

                   model StepChild{
                        id        String  @id @default(cuid())
                        s         String  @unique

                        parentOpt Parent?
                   }"""
      TestDataModels(mongo = dm1, sql = dm2)
    }

    testDataModels.testV11 { project =>
      server.query(
        """mutation {
        |  createParent(data: {
        |    p: "p1"
        |    childrenOpt: {
        |      create: [{c: "c1"},{c: "c2"}]
        |    }
        |    stepChildOpt: {
        |      create: {s: "s1"}
        |    }
        |  }){
        |    p
        |  }
        |}""".stripMargin,
        project
      )

      server.query(
        s"""
         |mutation {
         |  deleteManyParents(
         |    where: { p: { equals: "p1" } }
         | ) {
         |    count
         |  }
         |}
      """.stripMargin,
        project
      )
    }
  }
}
