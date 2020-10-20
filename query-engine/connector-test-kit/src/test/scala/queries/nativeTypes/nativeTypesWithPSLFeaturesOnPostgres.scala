package queries.nativeTypes

import org.scalatest.{FlatSpec, Matchers}
import util._

class nativeTypesWithPSLFeaturesOnPostgres extends FlatSpec with Matchers with ApiSpecBase {
 override def runOnlyForConnectors: Set[ConnectorTag] = Set(ConnectorTag.PostgresConnectorTag)


  "Using Prisma scalar type String with native types Char and VarChar and PSL features" should "work" in {
    val prisma_type = Vector("String")
    val native_type = Vector("Char(12)", "VarChar(12)", "Text", "Bit(6)", "Bit(1)", "VarBit(3)",  "VarBit(10)",  "Uuid")
    val default_arg = Vector("cuid()", "uuid()")
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
            |generator client {
            |  provider = "prisma-client-js"
            |  previewFeatures = ["nativeTypes"]
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
            |  name     $p_type @test.$n_type  @default($d_arg)
            |  @@unique([firstname, lastname])
            |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
         }
  }

  "Using Prisma scalar type Int and static default value with native types and PSL features" should "work" in {
    val prisma_type = Vector("Int")
    val native_type = Vector("Integer", "SmallInt", "BigInt")
    val default_arg = Vector(4, 20)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
           |generator client {
           |  provider = "prisma-client-js"
           |  previewFeatures = ["nativeTypes"]
           |}
           |
           |model House {
           |  id $p_type @test.$n_type @id
           |  name $p_type @test.$n_type  @default($d_arg)
           |}
    """.stripMargin
      }
    assert(database.setupWithStatusCode(project) == 0)
  }
  }

  "Using Prisma scalar type Int with native types and PSL features" should "work" in {
    val prisma_type = Vector("Int")
    val native_type = Vector("Integer", "SmallInt", "BigInt", "SmallSerial", "Serial", "BigSerial")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
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
             |  name $p_type @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type boolean with native type and PSL features" should "work" in {
    val prisma_type = Vector("Boolean")
    val native_type = Vector("Boolean")
    val default_arg = Vector("true", "false")
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type  @default($d_arg)
             |  optional $p_type? @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type Decimal with native type and PSL features" should "work" in {
    val prisma_type = Vector("Decimal")
    val native_type = Vector("Decimal(5,2)", "Numeric(4, 2)")
    val default_arg = Vector(999.99, -999.99)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
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
    val native_type = Vector("Real", "DoublePrecision")
    val default_arg = Vector(1.1, -999.99)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
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
    val native_type = Vector("ByteA")
    val default_arg = Vector(1, 0)
    for (p_type <- prisma_type;
         n_type <- native_type;
         d_arg <- default_arg
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
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

  "Using Prisma scalar type datetime with native type and PSL features" should "work" in {
    val prisma_type = Vector("DateTime")
    val native_type = Vector( "Time(1)", "Timestamp(2)", "TimestampWithTimeZone(2)",  "Date","TimeWithTimeZone(4)")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |  time $p_type @test.$n_type @updatedAt
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type Duration with native type and PSL features" should "work" in {
    val prisma_type = Vector("Duration")
    val native_type = Vector( "Interval(4)")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
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

  "Using Prisma scalar type JSON with native type and PSL features" should "work" in {
    val prisma_type = Vector("Json")
    val native_type = Vector("Json", "JsonB")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |  tuser    $p_type  @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

  "Using Prisma scalar type XML with native type and PSL features" should "work" in {
    val prisma_type = Vector("XML")
    val native_type = Vector("Xml")
    for (p_type <- prisma_type;
         n_type <- native_type
         )
      yield {
        val project = SchemaDsl.fromStringV11() {
          s"""
             |generator client {
             |  provider = "prisma-client-js"
             |  previewFeatures = ["nativeTypes"]
             |}
             |
             |model User {
             |  email    String  @unique
             |  name     $p_type @test.$n_type
             |  optional $p_type? @test.$n_type
             |  test   $p_type  @test.$n_type
             |}
    """.stripMargin
        }
        assert(database.setupWithStatusCode(project) == 0)
      }
  }

}
