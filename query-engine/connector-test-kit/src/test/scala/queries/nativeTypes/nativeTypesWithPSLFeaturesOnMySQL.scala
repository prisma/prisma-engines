package queries.nativeTypes

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorTag.MySqlConnectorTag
import util._

class nativeTypesWithPSLFeaturesOnMySQL extends FlatSpec with Matchers with ApiSpecBase {
 override def runOnlyForConnectors: Set[ConnectorTag] = Set(MySqlConnectorTag)


  "Using Prisma scalar type String with native types Char and VarChar and PSL features" should "work" in {
    val prisma_type = Vector("String")
    val native_type = Vector("Char(12)", "VarChar(12)")
    for (p_type <- prisma_type;
         n_type <- native_type)
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
            |model Item {
            |  id   $p_type @test.$n_type @id
            |  test $p_type @test.$n_type @unique
            |  optional $p_type? @test.$n_type
            |}
            |
            |model Post {
            |  firstName $p_type @test.$n_type
            |  lastName  $p_type @test.$n_type
            |  email     String @unique
            |  @@id([firstName, lastName])
            |}
            |
            |model SingleColumnIndex {
            |  id       Int     @id @default(autoincrement())
            |  title    $p_type @test.$n_type
            |  @@index([title])
            |}
            |
            |model MultiColumnIndex {
            |  id       Int     @id @default(autoincrement())
            |  title    $p_type @test.$n_type
            |  content  $p_type? @test.$n_type
            |  @@index([title, content])
            |}
            |
            |model User {
            |  id        Int     @default(autoincrement()) @id
            |  firstname $p_type @test.$n_type
            |  lastname $p_type @test.$n_type
            |  @@unique([firstname, lastname])
            |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
         }
  }

  "Using Prisma scalar type String with native types and default id attribute" should "work" in {
    val prisma_type = Vector("String")
    val native_type = Vector("Char(12)", "VarChar(12)", "Text", "MediumText", "LongText", "TinyText")
    val default_arg = Vector("cuid()", "uuid()")
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type  @default($d_arg)
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  /* // does not work: Error querying the database: Server error: `ERROR 42000 (1063): Incorrect column specifier for column 'id'
  "Using Prisma scalar type Int with native type Year" should "work" in {
    val project = SchemaDsl.fromStringV11() {
      s"""
         |model House {
         |  id   Int @test.Year @id  @default(autoincrement())
         |}
    """.stripMargin
    }
    assert(database.setupWithStatusCode(project) == 0)
  } */

  "Using Prisma scalar type Int with native types and PSL features" should "work" in {
    val prisma_type = Vector("Int")
    val native_type = Vector("Int", "UnsignedInt", "SmallInt", "UnsignedSmallInt", "MediumInt", "UnsignedMediumInt", "BigInt", "UnsignedBigInt", "Year")
    val default_arg = Vector(4, 20)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model Item {
             |  id   $p_type @test.$n_type @id
             |  test $p_type @test.$n_type @unique
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Post {
             |  firstName $p_type @test.$n_type
             |  lastName  $p_type @test.$n_type
             |  email     $p_type @test.$n_type @unique
             |  @@id([firstName, lastName])
             |}
             |
             |model User {
             |  id        Int     @default(autoincrement()) @id
             |  firstname $p_type @test.$n_type
             |  lastname $p_type @test.$n_type
             |  @@unique([firstname, lastname])
             |}
             |
             |model SingleColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  @@index([title])
             |}
             |
             |model MultiColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  content  $p_type? @test.$n_type
             |  @@index([title, content])
             |}
             |
             |model House {
             |  id $p_type @test.$n_type @id
             |  name     $p_type @test.$n_type  @default($d_arg)
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type boolean with native type and PSL features" should "work" in {
    val prisma_type = Vector("Boolean", "Int")
    val native_type = Vector("TinyInt")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type Decimal with native type and PSL features" should "work" in {
    val prisma_type = Vector("Decimal")
    val native_type = Vector("Decimal(5,2)")
    val default_arg = Vector(999.99, -999.99)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User2 {
             |  email    String  @unique
             |  name     $p_type @test.$n_type  @default($d_arg)
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Item {
             |  id   $p_type @test.$n_type @id
             |  test $p_type @test.$n_type @unique
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Post {
             |  firstName $p_type @test.$n_type
             |  lastName  $p_type @test.$n_type
             |  email     $p_type @test.$n_type @unique
             |  @@id([firstName, lastName])
             |}
             |
             |model User {
             |  id        Int     @default(autoincrement()) @id
             |  firstname $p_type @test.$n_type
             |  lastname $p_type @test.$n_type
             |  @@unique([firstname, lastname])
             |}
             |
             |model SingleColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  @@index([title])
             |}
             |
             |model MultiColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  content  $p_type? @test.$n_type
             |  @@index([title, content])
             |}
             |
             |model House {
             |  id $p_type @test.$n_type @id
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type Float with native types and PSL features" should "work" in {
    val prisma_type = Vector("Float")
    val native_type = Vector("Float", "Double")
    val default_arg = Vector(1.1, -999.99)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User2 {
             |  email    String  @unique
             |  name     $p_type @test.$n_type  @default($d_arg)
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Item {
             |  id   $p_type @test.$n_type @id
             |  test $p_type @test.$n_type @unique
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Post {
             |  firstName $p_type @test.$n_type
             |  lastName  $p_type @test.$n_type
             |  email     $p_type @test.$n_type @unique
             |  @@id([firstName, lastName])
             |}
             |
             |model User {
             |  id        Int     @default(autoincrement()) @id
             |  firstname $p_type @test.$n_type
             |  lastname $p_type @test.$n_type
             |  @@unique([firstname, lastname])
             |}
             |
             |model SingleColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  @@index([title])
             |}
             |
             |model MultiColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  content  $p_type? @test.$n_type
             |  @@index([title, content])
             |}
             |
             |model House {
             |  id $p_type @test.$n_type @id
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type Bytes with native types and PSL features" should "work" in {
    val prisma_type = Vector("Bytes")
    val native_type = Vector("Bit(1)", "Bit(5)", "Binary(5)", "Binary(10)", "VarBinary(10)", "TinyBlob", "Blob", "MediumBlob", "LongBlob")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User2 {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type Bytes with native types and index, id ad unique features" should "work" in {
    val prisma_type = Vector("Bytes")
    val native_type = Vector("Bit(1)", "Bit(5)", "Binary(5)", "Binary(10)", "VarBinary(10)")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User2 {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Item {
             |  id   $p_type @test.$n_type @id
             |  test $p_type @test.$n_type @unique
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Post {
             |  firstName $p_type @test.$n_type
             |  lastName  $p_type @test.$n_type
             |  email     $p_type @test.$n_type @unique
             |  @@id([firstName, lastName])
             |}
             |
             |model User {
             |  id        Int     @default(autoincrement()) @id
             |  firstname $p_type @test.$n_type
             |  lastname $p_type @test.$n_type
             |  @@unique([firstname, lastname])
             |}
             |
             |model SingleColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  @@index([title])
             |}
             |
             |model MultiColumnIndex {
             |  id       Int     @id @default(autoincrement())
             |  title    $p_type @test.$n_type
             |  content  $p_type? @test.$n_type
             |  @@index([title, content])
             |}
             |
             |model House {
             |  id $p_type @test.$n_type @id
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type datetime with native type and PSL features" should "work" in {
    val prisma_type = Vector("DateTime")
    val native_type = Vector("Time", "Time(2)", "Date", "Datetime", "Datetime(3)", "Timestamp", "Timestamp(3)")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User {
             |  id    Int  @id
             |  optional $p_type? @test.$n_type
             |}
             |
             |model Post {
             |   id Int @id
             |   time $p_type @test.$n_type @updatedAt
             |}
             |
             |model Sun {
             |  id Int @id
             |  name     $p_type @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  /*
  "Using Prisma scalar type datetime with native type and now as default" should "work" in { // does not work
    // [migration-engine/connectors/sql-migration-connector/src/sql_renderer/mysql_renderer.rs:233:39] internal error: entered unreachable code: NOW default on non-datetime column"
    val prisma_type = Vector("DateTime")
    val native_type = Vector("Time", "Time(2)", "Date", "Datetime", "Datetime(3)", "Timestamp", "Timestamp(3)")
    val default_arg = Vector("now()")
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model Sun {
             |  id Int @id
             |  name     $p_type @test.$n_type  @default($d_arg)
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  } */

  "Using Prisma scalar type JSON with native type and PSL features" should "work" in {
    val prisma_type = Vector("Json")
    val native_type = Vector("JSON")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |  time $p_type @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

}
