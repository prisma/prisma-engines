package writes.nestedMutations

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class CombiningDifferentNestedMutationsSpec extends FlatSpec with Matchers with ApiSpecBase with SchemaBaseV11 {
  override def runOnlyForCapabilities: Set[ConnectorCapability] = Set(JoinRelationLinksCapability)
  //hardcoded execution order
  //  nestedCreates
  //  nestedUpdates
  //  nestedUpserts
  //  nestedDeletes
  //  nestedConnects
  //  nestedSets
  //  nestedDisconnects
  //  nestedUpdateManys
  //  nestedDeleteManys
  // this could be extended to more combinations and to different schemata
  // the error behavior would be interesting to test, which error is returned, does rollback work

  "A create followed by an update" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        """
          |mutation {
          |  createParent(
          |    data: { p: "p1", p_1: "1", p_2: "2" childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "q1t", c_2: "asd" }] } }
          |  ) {
          |    childrenOpt(orderBy: { c: asc }) {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      val res2 = server.query(
        """
          |mutation {
          |  updateParent(
          |    where: { p: "p1" }
          |    data: {
          |      childrenOpt: {
          |        create: [{ c: "c3", c_1: "jeesus", c_2: "maria" }, { c: "c4", c_1: "3t1", c_2: "a1" }]
          |        update: [{ where: { c: "c3" }, data: { c: { set: "cUpdated" } } }]
          |      }
          |    }
          |  ) {
          |    childrenOpt(orderBy: { c: asc }) {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"},{"c":"cUpdated"}]}}}""")

      server.query(s"""query{children(orderBy: { c: asc }){c, parentsOpt(orderBy: { p: asc }){p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]},{"c":"cUpdated","parentsOpt":[{"p":"p1"}]}]}}""")

    }
  }

  "A create followed by a delete" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        """
          |mutation {
          |  createParent(
          |    data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "zol", c_2: "lol" }] } }
          |  ) {
          |    childrenOpt {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      val res2 = server.query(
        """
          |mutation {
          |  updateParent(
          |    where: { p: "p1" }
          |    data: {
          |      childrenOpt: { create: [{ c: "c3", c_1: "yksi", c_2: "kaksi" }, { c: "c4", c_1: "kolme", c_2: "nelja" }], delete: [{ c: "c3" }] }
          |    }
          |  ) {
          |    childrenOpt {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]}]}}""")
    }
  }

  "A create followed by a set" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        """
          |mutation {
          |  createParent(
          |    data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "om", c_2: "mo" }] } }
          |  ) {
          |    childrenOpt {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      val res2 = server.query(
        """
          |mutation {
          |  updateParent(
          |    where: { p: "p1" }
          |    data: {
          |      childrenOpt: { create: [{ c: "c3", c_1: "yksi", c_2: "kaksi" }, { c: "c4", c_1: "kolme", c_2: "neljae" }], set: [{ c: "c3" }] }
          |    }
          |  ) {
          |    childrenOpt {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c3"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[]},{"c":"c2","parentsOpt":[]},{"c":"c3","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[]}]}}""")

    }
  }

  "A create followed by an upsert" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        """
          |mutation {
          |  createParent(
          |    data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "1", c_2: "2" }, { c: "c2", c_1: "3", c_2: "4" }] } }
          |  ) {
          |    childrenOpt(orderBy: { c: asc }) {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      val res2 = server.query(
        """
          |mutation {
          |  updateParent(
          |    where: { p: "p1" }
          |    data: {
          |      childrenOpt: {
          |        create: [{ c: "c3", c_1: "5", c_2: "6" }, { c: "c4", c_1: "7", c_2: "8" }]
          |        upsert: [
          |          {
          |            where: { c: "c3" }
          |            create: { c: "should not matter", c_1: "no matter", c_2: "matter not" }
          |            update: { c: { set: "cUpdated" }}
          |          }
          |          {
          |            where: { c: "c5" }
          |            create: { c: "cNew", c_1: "matter", c_2: "most" }
          |            update: { c: { set: "should not matter" }}
          |          }
          |        ]
          |      }
          |    }
          |  ) {
          |    childrenOpt(orderBy: { c: asc }) {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"},{"c":"cNew"},{"c":"cUpdated"}]}}}""")

      server.query(s"""query{children(orderBy: { c: asc }){c, parentsOpt(orderBy: { p: asc }){p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c4","parentsOpt":[{"p":"p1"}]},{"c":"cNew","parentsOpt":[{"p":"p1"}]},{"c":"cUpdated","parentsOpt":[{"p":"p1"}]}]}}""")
    }
  }

  "A create followed by a disconnect" should "work" in {
    schemaWithRelation(onParent = ChildList, onChild = ParentList).test { t =>
      val project = SchemaDsl.fromStringV11() {
        t.datamodel
      }
      database.setup(project)

      val res = server.query(
        """
          |mutation {
          |  createParent(
          |    data: { p: "p1", p_1: "1", p_2: "2", childrenOpt: { create: [{ c: "c1", c_1: "foo", c_2: "bar" }, { c: "c2", c_1: "asd", c_2: "qawf" }] } }
          |  ) {
          |    childrenOpt {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res.toString should be("""{"data":{"createParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"}]}}}""")

      val res2 = server.query(
        """
          |mutation {
          |  updateParent(
          |    where: { p: "p1" }
          |    data: {
          |      childrenOpt: {
          |        create: [{ c: "c3", c_1: "yksi", c_2: "kaksi" }, { c: "c4", c_1: "kolme", c_2: "neljae" }]
          |        disconnect: [{ c: "c3" }]
          |      }
          |    }
          |  ) {
          |    childrenOpt {
          |      c
          |    }
          |  }
          |}
        """.stripMargin,
        project
      )

      res2.toString should be("""{"data":{"updateParent":{"childrenOpt":[{"c":"c1"},{"c":"c2"},{"c":"c4"}]}}}""")

      server.query(s"""query{children{c, parentsOpt{p}}}""", project).toString should be(
        """{"data":{"children":[{"c":"c1","parentsOpt":[{"p":"p1"}]},{"c":"c2","parentsOpt":[{"p":"p1"}]},{"c":"c3","parentsOpt":[]},{"c":"c4","parentsOpt":[{"p":"p1"}]}]}}""")
    }
  }
}
