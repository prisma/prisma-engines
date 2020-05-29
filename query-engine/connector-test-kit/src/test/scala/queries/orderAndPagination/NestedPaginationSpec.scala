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
        |  tops{t, middles{m, bottoms{b}}}
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
  // WIP - This requires fixes
//  "Middle level cursor" should "return all items after and including the cursor and return nothing for other tops" in {
//    testDataModels.testV11 { project =>
//      createData(project)
//      val result = server.query(
//        """
//          |{
//          |  tops{t, middles(cursor: { m: "M22" }){ m }}
//          |
//          |}
//        """,
//        project
//      )
//
//      //{"data":{"tops":[{"t":"T1","middles":[]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M31"},{"m":"M32"},{"m":"M33"}]}]}}
//      result.toString() should be(
//        """{"data":{"tops":[{"t":"T1","middles":[{"m":"M12"},{"m":"M13"}]},{"t":"T2","middles":[{"m":"M22"},{"m":"M23"}]},{"t":"T3","middles":[{"m":"M32"},{"m":"M33"}]}]}}""")
//    }
//  }

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
        |  tops{t, middles(take: -1){m}}
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
        |  tops{t, middles(take: -3){m}}
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
        |  tops{t, middles(take: -4){m}}
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
        |  tops{middles{bottoms(take:-1){b}}}
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
        |  tops{middles{bottoms(take: -3){b}}}
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
        |  tops{middles{bottoms(take: -4){b}}}
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
        |  tops(skip: 1, take: -1){t, middles{m}}
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
        |  tops(skip: 1, take: -3){t, middles{m}}
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
        |  tops{t, middles(skip: 1, take: -1){m}}
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
        |  tops{t, middles(skip: 1, take: -3){m}}
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
        |  tops{t, middles(orderBy: m_DESC, take: 1){m}}
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
        |  tops{t, middles(orderBy: m_DESC, take: 3){m}}
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
}
