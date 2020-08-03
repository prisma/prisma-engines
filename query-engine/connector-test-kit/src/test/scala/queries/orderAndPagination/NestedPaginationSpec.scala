package queries.orderAndPagination

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.Json
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class NestedPaginationSpec extends FlatSpec with Matchers with ApiSpecBase {
  val testDataModels = {
    val s1 = """
      model Top {
        id      String   @id @default(cuid())
        t       String   @unique

        middles Middle[]
      }

      model Middle {
        id     String   @id @default(cuid())
        m      String   @unique
        top_id String

        top     Top      @relation(fields: [top_id], references: [id])
        bottoms Bottom[]
      }

      model Bottom {
        id        String @id @default(cuid())
        b         String @unique
        middle_id String

        middle Middle @relation(fields: [middle_id], references: [id])
      }
    """

    val s2 = """
      model Top {
        id     String   @id @default(cuid())
        t      String   @unique

        middles Middle[]
      }

      model Middle {
        id      String   @id @default(cuid())
        m       String   @unique
        top_id String

        top     Top      @relation(fields: [top_id], references: [id])
        bottoms Bottom[]
      }

      model Bottom {
        id        String @id @default(cuid())
        b         String @unique
        middle_id String

        middle Middle @relation(fields: [middle_id], references: [id])
      }
    """

    TestDataModels(mongo = Vector(s1), sql = Vector(s2))
  }

  "All data" should "be there" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles{ m, bottoms {b}}}
        |}
      """,
        project
      )

      result should be(Json.parse(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11","bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"m":"M12","bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"m":"M13","bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"t":"T2","middles":[{"m":"M21","bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"m":"M22","bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"m":"M23","bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"t":"T3","middles":[{"m":"M31","bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"m":"M32","bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"m":"M33","bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"""))
    }
  }

  /******************
    * Cursor tests. *
    *****************/
  "Middle level cursor" should "return all items after and including the cursor and return nothing for other tops" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
          |{
          |  tops{t, middles(cursor: { m: "M22" }, orderBy: { id: ASC }){ m }}
          |
          |}
        """,
        project
      )

      result.toString() should be("""{"data":{"tops":[{"t":"T1","middles":[]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[]}]}}""")
    }
  }

  /****************
    * Skip tests. *
    ***************/
  "Middle level skip 1" should "skip the first item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 1){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Middle level skip 3" should "skip all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 3){m}}
        |}
      """,
        project
      )

      result.toString() should be("""{"data":{"tops":[{"t":"T1","middles":[]},{"t":"T2","middles":[]},{"t":"T3","middles":[]}]}}""")
    }
  }

  "Middle level skip 4" should "skip all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 4){m}}
        |}
      """,
        project
      )

      result.toString() should be("""{"data":{"tops":[{"t":"T1","middles":[]},{"t":"T2","middles":[]},{"t":"T3","middles":[]}]}}""")
    }
  }

  "Bottom level skip 0" should "skip no items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(skip: 0){b}}}
        |}
      """,
        project
      )

      result should be(Json.parse(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}"""))
    }
  }

  "Bottom level skip 1" should "skip the first item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(skip:1){b}}}
        |}
      """,
        project
      )

      result should be(Json.parse(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B332"},{"b":"B333"}]}]}]}}"""))
    }
  }

  "Bottom level skip 3" should "skip all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(skip: 3){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]}]}}""")
    }
  }

  "Bottom level skip 4" should "skip all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(skip: 4){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]}]}}""")
    }
  }

  /**************
    * Take tests *
   **************/
  "Middle level take 0" should "return no items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: 0){m}}
        |}
      """,
        project
      )

      result.toString() should be("""{"data":{"tops":[{"t":"T1","middles":[]},{"t":"T2","middles":[]},{"t":"T3","middles":[]}]}}""")
    }
  }

  "Middle level take 1" should "return the first item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: 1){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"}]},{"t":"T2","middles":[{"m":"M21"}]},{"t":"T3","middles":[{"m":"M31"}]}]}}""")
    }
  }

  "Middle level take 3" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: 3){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Middle level take 4" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: 4){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Bottom level take 0" should "return no items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take: 0){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]},{"middles":[{"bottoms":[]},{"bottoms":[]},{"bottoms":[]}]}]}}""")
    }
  }

  "Bottom level take 1" should "return the first item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take:1){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B111"}]},{"bottoms":[{"b":"B121"}]},{"bottoms":[{"b":"B131"}]}]},{"middles":[{"bottoms":[{"b":"B211"}]},{"bottoms":[{"b":"B221"}]},{"bottoms":[{"b":"B231"}]}]},{"middles":[{"bottoms":[{"b":"B311"}]},{"bottoms":[{"b":"B321"}]},{"bottoms":[{"b":"B331"}]}]}]}}""")
    }
  }

  "Bottom level take 3" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take: 3){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}""")
    }

  }

  "Bottom level take 4" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take: 4){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}""")
    }
  }

  "Middle level take -1" should "return the last item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: -1, orderBy: { id: ASC }){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M13"}]},{"t":"T2","middles":[{"m":"M23"}]},{"t":"T3","middles":[{"m":"M33"}]}]}}""")
    }
  }

  "Middle level take -3" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: -3, orderBy: { id: ASC }) {m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Middle level take -4" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(take: -4, orderBy: { id: ASC }){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Bottom level take -1" should "return the last item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take: -1, orderBy: { id: ASC }){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B113"}]},{"bottoms":[{"b":"B123"}]},{"bottoms":[{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B213"}]},{"bottoms":[{"b":"B223"}]},{"bottoms":[{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B313"}]},{"bottoms":[{"b":"B323"}]},{"bottoms":[{"b":"B333"}]}]}]}}""")
    }
  }

  "Bottom level take -3" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take: -3, orderBy: { id: ASC }){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}""")
    }
  }

  "Bottom level take -4" should "return all items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{middles{bottoms(take: -4, orderBy: { id: ASC }){b}}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"middles":[{"bottoms":[{"b":"B111"},{"b":"B112"},{"b":"B113"}]},{"bottoms":[{"b":"B121"},{"b":"B122"},{"b":"B123"}]},{"bottoms":[{"b":"B131"},{"b":"B132"},{"b":"B133"}]}]},{"middles":[{"bottoms":[{"b":"B211"},{"b":"B212"},{"b":"B213"}]},{"bottoms":[{"b":"B221"},{"b":"B222"},{"b":"B223"}]},{"bottoms":[{"b":"B231"},{"b":"B232"},{"b":"B233"}]}]},{"middles":[{"bottoms":[{"b":"B311"},{"b":"B312"},{"b":"B313"}]},{"bottoms":[{"b":"B321"},{"b":"B322"},{"b":"B323"}]},{"bottoms":[{"b":"B331"},{"b":"B332"},{"b":"B333"}]}]}]}}""")
    }
  }

  /**********************
    * Skip + Take tests *
    *********************/
  "Top level skip 1 take 1" should "return the second item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops(skip: 1, take: 1){t, middles{m}}
        |}
      """,
        project
      )

      result.toString() should be("""{"data":{"tops":[{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]}]}}""")
    }
  }

  "Top level  skip 1 take 3" should "return only the last two items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops(skip: 1, take: 3){t, middles{m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Middle level skip 1 take 1" should "return the second" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 1, take: 1){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M12"}]},{"t":"T2","middles":[{"m":"M22"}]},{"t":"T3","middles":[{"m":"M32"}]}]}}""")
    }
  }

  "Middle level skip 1 take 3" should "return the last two items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 1, take: 3){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M32"},{"m":"M33"}]}]}}""")
    }
  }

  "Top level skip 1 take -1" should "return the second item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops(skip: 1, take: -1, orderBy: { id: ASC }){t, middles{m}}
        |}
      """,
        project
      )

      result.toString() should be("""{"data":{"tops":[{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]}]}}""")
    }
  }

  "Top level skip 1 take -3" should "return only the first two items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops(skip: 1, take: -3, orderBy: { id: ASC }){t, middles{m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"},{"m":"M23"}]}]}}""")
    }
  }

  "Middle level skip 1 take -1" should "return the second" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 1, take: -1, orderBy: { id: ASC }){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M12"}]},{"t":"T2","middles":[{"m":"M22"}]},{"t":"T3","middles":[{"m":"M32"}]}]}}""")
    }
  }

  "Middle level skip 1 take -3" should "return the first two items" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(skip: 1, take: -3, orderBy: { id: ASC }){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M11"},{"m":"M12"}]},{"t":"T2","middles":[{"m":"M21"},{"m":"M22"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"}]}]}}""")
    }
  }

  /*************************
    * Skip + take + order. *
    ************************/
  "Middle orderBy take 1" should "return the last item" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(orderBy: { m: DESC }, take: 1){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M13"}]},{"t":"T2","middles":[{"m":"M23"}]},{"t":"T3","middles":[{"m":"M33"}]}]}}""")
    }
  }

  "Middle level orderBy take 3" should "return all items in reverse order" in {
    testDataModels.testV11 { project =>
      createData(project)
      val result = server.query(
        """
        |{
        |  tops{t, middles(orderBy: { m: DESC }, take: 3){m}}
        |}
      """,
        project
      )

      result.toString() should be(
        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M13"},{"m":"M12"},{"m":"M11"}]},{"t":"T2","middles":[{"m":"M23"},{"m":"M22"},{"m":"M21"}]},{"t":"T3","middles":[{"m":"M33"},{"m":"M32"},{"m":"M31"}]}]}}""")
    }
  }

  private def createData(project: Project): Unit = {
    server.query(
      """
        |mutation {
        |  createTop(data: {t: "T1" middles:{create:[
        |     {m: "M11" bottoms:{create:[
        |         {b:"B111"}
        |         {b:"B112"}
        |         {b:"B113"}
        |     ]}
        |     },
        |     {m: "M12" bottoms:{create:[
        |         {b:"B121"}
        |         {b:"B122"}
        |         {b:"B123"}
        |     ]}
        |     },
        |     {m: "M13" bottoms:{create:[
        |         {b:"B131"}
        |         {b:"B132"}
        |         {b:"B133"}
        |     ]}
        |     }
        |  ]}}){ id }
        |}""".stripMargin,
      project
    )

    server.query(
      """
        |mutation {
        |  createTop(data: {t: "T2" middles:{create:[
        |     {m: "M21" bottoms:{create:[
        |         {b:"B211"}
        |         {b:"B212"}
        |         {b:"B213"}
        |     ]}
        |     },
        |     {m: "M22" bottoms:{create:[
        |         {b:"B221"}
        |         {b:"B222"}
        |         {b:"B223"}
        |     ]}
        |     },
        |     {m: "M23" bottoms:{create:[
        |         {b:"B231"}
        |         {b:"B232"}
        |         {b:"B233"}
        |     ]}
        |     }
        |  ]}}){ id }
        |}""".stripMargin,
      project
    )

    server.query(
      """
        |mutation {
        |  createTop(data: {t: "T3" middles:{create:[
        |     {m: "M31" bottoms:{create:[
        |         {b:"B311"}
        |         {b:"B312"}
        |         {b:"B313"}
        |     ]}
        |     },
        |     {m: "M32" bottoms:{create:[
        |         {b:"B321"}
        |         {b:"B322"}
        |         {b:"B323"}
        |     ]}
        |     },
        |     {m: "M33" bottoms:{create:[
        |         {b:"B331"}
        |         {b:"B332"}
        |         {b:"B333"}
        |     ]}
        |     }
        |  ]}}){ id }
        |}
      """,
      project
    )
  }

  /***************
    * M:N tests. *
    **************/
  // Special case: m:n relations, child is connected to many parents, using cursor pagination
  // A1 <> B1, B2, B3
  // A2 <> B2
  // A3
  "A many-to-many relationship with multiple connected children" should "return all items correctly with nested cursor pagination" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id    String   @id
        |  manyB ModelB[]
        |}
        |
        |model ModelB {
        |  id    String   @id
        |  manyA ModelA[]
        |}
      """.stripMargin
    }
    database.setup(project)

    var result = server.query(
      s"""mutation {
         |  createOneModelA(
         |    data: {
         |      id: "A1"
         |      manyB: {
         |        connectOrCreate: [
         |          { where: { id: "B1" }, create: { id: "B1" } }
         |          { where: { id: "B2" }, create: { id: "B2" } }
         |          { where: { id: "B3" }, create: { id: "B3" } }
         |        ]
         |      }
         |    }
         |  ) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
         |
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"createOneModelA":{"id":"A1","manyB":[{"id":"B1"},{"id":"B2"},{"id":"B3"}]}}}""")

    result = server.query(
      s"""mutation {
         |  createOneModelA(
         |    data: {
         |      id: "A2"
         |      manyB: {
         |        connectOrCreate: [
         |          { where: { id: "B2" }, create: { id: "B2" } }
         |        ]
         |      }
         |    }
         |  ) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
         |
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"createOneModelA":{"id":"A2","manyB":[{"id":"B2"}]}}}""")

    result = server.query(
      s"""mutation{
         |  createOneModelA(data: {
         |    id: "A3"
         |  }) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"createOneModelA":{"id":"A3","manyB":[]}}}""")

    result = server.query(
      s"""{
         |  findManyModelA {
         |    id
         |    manyB(cursor: {
         |      id: "B2"
         |    }) {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be(
      """{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B2"},{"id":"B3"}]},{"id":"A2","manyB":[{"id":"B2"}]},{"id":"A3","manyB":[]}]}}""")
  }

  // Special case: m:n relations, child is connected to many parents, using cursor pagination
  // A1 <> B1, B2, B3, B4, B5, B6
  // A2 <> B2, B3, B5, B7, B8
  // A3
  "A many-to-many relationship with multiple connected children" should "return all items correctly with nested cursor pagination and skip / take" in {
    val project = SchemaDsl.fromStringV11() {
      """
        |model ModelA {
        |  id    String   @id
        |  manyB ModelB[]
        |}
        |
        |model ModelB {
        |  id    String   @id
        |  manyA ModelA[]
        |}
      """.stripMargin
    }
    database.setup(project)

    // >>> Begin create test data
    var result = server.query(
      s"""mutation {
         |  createOneModelA(
         |    data: {
         |      id: "A1"
         |      manyB: {
         |        connectOrCreate: [
         |          { where: { id: "B1" }, create: { id: "B1" } }
         |          { where: { id: "B2" }, create: { id: "B2" } }
         |          { where: { id: "B3" }, create: { id: "B3" } }
         |          { where: { id: "B4" }, create: { id: "B4" } }
         |          { where: { id: "B5" }, create: { id: "B5" } }
         |          { where: { id: "B6" }, create: { id: "B6" } }
         |        ]
         |      }
         |    }
         |  ) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
         |
      """,
      project,
      legacy = false,
    )

    result.toString() should be(
      """{"data":{"createOneModelA":{"id":"A1","manyB":[{"id":"B1"},{"id":"B2"},{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]}}}""")

    result = server.query(
      s"""mutation {
         |  createOneModelA(
         |    data: {
         |      id: "A2"
         |      manyB: {
         |        connectOrCreate: [
         |          { where: { id: "B2" }, create: { id: "B2" } },
         |          { where: { id: "B3" }, create: { id: "B3" } }
         |          { where: { id: "B5" }, create: { id: "B5" } }
         |          { where: { id: "B7" }, create: { id: "B7" } }
         |          { where: { id: "B8" }, create: { id: "B8" } }
         |        ]
         |      }
         |    }
         |  ) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
         |
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"createOneModelA":{"id":"A2","manyB":[{"id":"B2"},{"id":"B3"},{"id":"B5"},{"id":"B7"},{"id":"B8"}]}}}""")

    result = server.query(
      s"""mutation{
         |  createOneModelA(data: {
         |    id: "A3"
         |  }) {
         |    id
         |    manyB {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    result.toString() should be("""{"data":{"createOneModelA":{"id":"A3","manyB":[]}}}""")

    // <<< End create test data

    result = server.query(
      s"""{
         |  findManyModelA {
         |    id
         |    manyB(cursor: {
         |      id: "B2"
         |    }, skip: 1) {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    // Cursor is B2. We skip 1, so B2 is not included. This makes:
    // A1 => [B3, B4, B5, B6]
    // A2 => [B3, B5, B7, B8]
    // A3 => []
    result.toString() should be(
      """{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B3"},{"id":"B4"},{"id":"B5"},{"id":"B6"}]},{"id":"A2","manyB":[{"id":"B3"},{"id":"B5"},{"id":"B7"},{"id":"B8"}]},{"id":"A3","manyB":[]}]}}""")

    result = server.query(
      s"""{
         |  findManyModelA {
         |    id
         |    manyB(cursor: {
         |      id: "B2"
         |    }, skip: 1, take: 2) {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    // Cursor is B2. We skip 1, so B2 is not included, and take the next 2. This makes:
    // A1 => [B3, B4]
    // A2 => [B3, B5]
    // A3 => []
    result.toString() should be(
      """{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B3"},{"id":"B4"}]},{"id":"A2","manyB":[{"id":"B3"},{"id":"B5"}]},{"id":"A3","manyB":[]}]}}""")

    result = server.query(
      s"""{
         |  findManyModelA {
         |    id
         |    manyB(cursor: {
         |      id: "B5"
         |    }, skip: 1, take: -2, orderBy: { id: ASC }) {
         |      id
         |    }
         |  }
         |}
      """,
      project,
      legacy = false,
    )

    // Cursor is B5. We skip 1, so B5 is not included, and take the previous 2 records. This makes:
    // A1 => [B3, B4]
    // A2 => [B2, B3]
    // A3 => []
    result.toString() should be(
      """{"data":{"findManyModelA":[{"id":"A1","manyB":[{"id":"B3"},{"id":"B4"}]},{"id":"A2","manyB":[{"id":"B2"},{"id":"B3"}]},{"id":"A3","manyB":[]}]}}""")
  }
}
