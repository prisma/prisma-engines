package queries.nativeTypes

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class nativeTypesWithPSLFeaturesOnMySQL extends FlatSpec with Matchers with ApiSpecBase {
 override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag)


  "Using Prisma scalar type String with native types and PSL features" should "be successful" in {
    val prisma_type = Vector("String")
    val native_type = Vector("LongText", "TinyText", "Char(12)", "Varchar(12)")
    for (p_type <- prisma_type;
         n_type <- native_type)
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
            |generator client {
            |  provider = "prisma-client-js"
            |  previewFeatures = ["nativeTypes"]
            |}
            |
            |model Item {
            |  id    String @id
            |  test $p_type @test.$n_type @unique
            |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
         }
  }
}
