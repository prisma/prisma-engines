package writes.dataTypes.native_tyes

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class MySqlNativeTypesSpec extends FlatSpec with Matchers with ApiSpecBase with ConnectorAwareTest {
  override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag)

  "MySQL native types" should "work" in {
    val project = ProjectDsl.fromString {
      """
        |model Model {
        |  id  String @id @default(cuid())
        |
        |  int  Int @test.Int
        |  sInt Int @test.SmallInt
        |  tInt Int @test.TinyInt
        |  mInt Int @test.MediumInt
        |  bInt Int @test.BigInt
        |
        |  float    Float @test.Float
        |  dfloat   Float @test.Double
        |
        |  decFloat Decimal @test.Decimal(2, 1)
        |  numFloat Decimal @test.Numeric(2, 1)
        |
        |  char  String @test.Char(55)
        |  vChar String @test.VarChar(65)
        |  tText String @test.TinyText
        |  text  String @test.Text
        |  mText String @test.MediumText
        |  ltext String @test.LongText
        |
        |  date  DateTime @test.Date
        |  time  DateTime @test.Time(5)
        |  dtime DateTime @test.Datetime
        |  ts    DateTime @test.Timestamp
        |  year  Int @test.Year
        |
        |  json Json @test.JSON
        |}"""
    }

    database.setup(project)
  }
}
