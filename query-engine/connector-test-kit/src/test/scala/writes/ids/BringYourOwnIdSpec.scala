package writes.ids

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag._
import util._

class BringYourOwnIdSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {

  override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag, PostgresConnectorTag, SQLiteConnectorTag)

  "A Create Mutation" should "create and return item with own Id" in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      val res = server.query(
        s"""mutation {
         |  createParent(data: {p: "Parent", id: "Own Id"}){p, id}
         |}""",
        project = project
      )

      res.toString should be(s"""{"data":{"createParent":{"p":"Parent","id":"Own Id"}}}""")

      val errorTarget = () match {
        case _ if connectorTag == ConnectorTag.MySqlConnectorTag => "constraint: `PRIMARY`"
        case _                                                   => "fields: (`id`)"
      }

      server.queryThatMustFail(
        s"""mutation {
         |  createParent(data: {p: "Parent2", id: "Own Id"}){p, id}
         |}""",
        project = project,
        errorCode = 2002,
        errorContains = s"Unique constraint failed on the $errorTarget"
      )
    }
  }

  "A Create Mutation" should "error for id that is invalid" ignore { //Fixme does that make sense??
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      val res = server.query(
        s"""mutation {
         |  createParent(data: {p: "Parent", id: 12}){p, id}
         |}""",
        project = project
      )

      res.toString should be(s"""{"data":{"createParent":{"p":"Parent","id":"12"}}}""")
    }
  }

  "A Create Mutation" should "error for id that is invalid 2" in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      server.queryThatMustFail(
        s"""mutation {
         |  createParent(data: {p: "Parent", id: true}){p, id}
         |}""",
        project = project,
        errorCode = 2009,
        errorContains = """`Mutation.createParent.data.ParentCreateInput.id`: Value types mismatch. Have: Boolean(true), want: String"""
      )
    }
  }

  "A Create Mutation" should "error for id that is invalid 3" taggedAs (IgnoreSQLite, IgnorePostgres, IgnoreMySql) in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      server.queryThatMustFail(
        s"""mutation {
         |  createParent(data: {p: "Parent", id: "this is probably way to long, lets see what error it throws"}){p, id}
         |}""",
        project = project,
        errorCode = 3007,
        errorContains = "Value for field id is too long."
      )
    }
  }

  "A Nested Create Mutation" should "create and return item with own Id" in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      val res = server.query(
        s"""mutation {
         |  createParent(data: {p: "Parent", id: "Own Id", childOpt:{create:{c:"Child", id: "Own Child Id"}}}){p, id, childOpt { c, id} }
         |}""",
        project = project
      )

      res.toString should be(s"""{"data":{"createParent":{"p":"Parent","id":"Own Id","childOpt":{"c":"Child","id":"Own Child Id"}}}}""")

      val constraintTarget = () match {
        case _ if connectorTag == ConnectorTag.MySqlConnectorTag => "constraint: `PRIMARY`"
        case _                                                   => "fields: (`id`)"
      }

      server.queryThatMustFail(
        s"""mutation {
         |createParent(data: {p: "Parent 2", id: "Own Id 2", childOpt:{create:{c:"Child 2", id: "Own Child Id"}}}){p, id, childOpt { c, id} }
         |}""",
        project = project,
        errorCode = 2002,
        errorContains = s"Unique constraint failed on the $constraintTarget"
      )
    }
  }

  "A Nested Create Mutation" should "error with invalid id" taggedAs (IgnoreSQLite, IgnorePostgres, IgnoreMySql) in { // TODO: Should we really validate this
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      server.queryThatMustFail(
        s"""mutation {
         |createParent(data: {p: "Parent 2", id: "Own Id 2", childOpt:{create:{c:"Child 2", id: "This is way too long and should error"}}}){p, id, childOpt { c, id} }
         |}""",
        project = project,
        errorCode = 3007,
        errorContains = "Value for field id is too long."
      )
    }
  }

  "An Upsert Mutation" should "work" in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      val res = server.query(
        s"""mutation {
         |upsertParent(
         |    where: {id: "Does not exist"}
         |    create: {p: "Parent 2", id: "Own Id"}
         |    update: {p: { set: "Parent 2"} }
         |    )
         |  {p, id}
         |}""",
        project = project
      )

      res.toString should be("""{"data":{"upsertParent":{"p":"Parent 2","id":"Own Id"}}}""")
    }
  }

  "An Upsert Mutation" should "error with id that is too long" taggedAs (IgnoreSQLite, IgnorePostgres, IgnoreMySql) in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      server.queryThatMustFail(
        s"""mutation {
         |upsertParent(
         |    where: {id: "Does not exist"}
         |    create: {p: "Parent 2", id: "Way way too long for a proper id"}
         |    update: {p: { set: "Parent 2" }}
         |    )
         |  {p, id}
         |}""",
        project = project,
        errorCode = 3007,
        errorContains = "Value for field id is too long."
      )
    }
  }

  "An nested Upsert Mutation" should "work" in {
    schemaP1optToC1opt.test { dataModel =>
      val project = ProjectDsl.fromString { dataModel }
      database.setup(project)

      val res = server.query(
        s"""mutation {
         |  createParent(data: {p: "Parent", id: "Own Id"}){p, id}
         |}""",
        project = project
      )

      res.toString should be(s"""{"data":{"createParent":{"p":"Parent","id":"Own Id"}}}""")

      val res2 = server.query(
        s"""mutation {
         |updateParent(
         |    where: {id: "Own Id"}
         |    data: {
         |        childOpt: {upsert:{
         |              create:{ id: "Own Id 3", c: "test 3"}
         |              update:{ c: { set: "Does not matter" } }
         |        }}
         |      }
         |    )
         |  {p, id, childOpt{c, id}}
         |}""",
        project = project
      )

      res2.toString should be("""{"data":{"updateParent":{"p":"Parent","id":"Own Id","childOpt":{"c":"test 3","id":"Own Id 3"}}}}""")
    }
  }

  "An id field with a custom name" should "work" in {
    val dataModel =
      """
        |model Blog {
        |  myId String @id @default(cuid())
        |  name String
        |}
      """.stripMargin

    val project = ProjectDsl.fromString { dataModel }
    database.setup(project)

    val res = server.query(
      s"""mutation {
         |  createBlog(data: {name: "MyBlog"}){ name }
         |}""",
      project = project
    )
    res should be("""{"data": { "createBlog": { "name": "MyBlog" } } }""".parseJson)
  }
}
