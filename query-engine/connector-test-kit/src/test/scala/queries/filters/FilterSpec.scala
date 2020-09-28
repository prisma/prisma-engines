package queries.filters

import org.scalatest.{FlatSpec, Matchers}
import util.ConnectorCapability.JoinRelationLinksCapability
import util._

class FilterSpec extends FlatSpec with Matchers with ApiSpecBase {
  override def runOnlyForCapabilities = Set(JoinRelationLinksCapability)

  val project: Project = ProjectDsl.fromString { """
     |model User {
     |  id         String   @id @default(cuid())
     |  unique     Int      @unique
     |  name       String?
     |  optional   String?
     |  vehicle_id String?
     |
     |  ride Vehicle? @relation(fields: [vehicle_id], references: [id])
     |}
     |
     |model Vehicle {
     |  id     String  @id @default(cuid())
     |  unique Int     @unique
     |  brand  String?
     |  parked Boolean?
     |
     |  owner  User
     |}
     |
     |model ParkingLot {
     |  id       String @id @default(cuid())
     |  unique   Int    @unique
     |  area     String?
     |  size     Float?
     |  capacity Int?
     |}""".stripMargin }

  override protected def beforeAll(): Unit = {
    super.beforeAll()
    database.setup(project)
    populate
  }

  "Queries" should "display all items if no filter is given" in {
    val filter = ""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
    vehicleUniques(filter) should be(Vector(1, 2, 3))
    lotUniques(filter) should be(Vector(1, 2))
  }

  "Simple filter" should "work" in {
    val filter = """(where: { name: { equals: "John" }})"""

    userUniques(filter) should be(Vector(4))
  }

  "Inverted simple filter" should "work" in {
    val filter = """(where: { name: { not: { equals: "John" }}})"""

    userUniques(filter) should be(Vector(1, 2, 3))
  }

  "Inverted simple filter" should "work with an implicit not equals" in {
    val filter = """(where: { name: { not: "John" }})"""

    userUniques(filter) should be(Vector(1, 2, 3))
  }

  "Simple filter" should "work with an implicit equals" in {
    val filter = """(where: { name: "John" })"""

    userUniques(filter) should be(Vector(4))
  }

  "Simple filter" should "work with an implicit equals null for a nullable field" in {
    val filter = """(where: { name: null })"""

    userUniques(filter) should be(Vector())
  }

  "Using in with null" should "return all nodes with null for that field" in {
    val filter = """(where: {optional: { in: null }})"""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
  }

  "Using in with [null]" should "return all nodes with null for that field" ignore {
    val filter = """(where: {optional: { in: ["test", null] }})"""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
  }

  "Using in" should "return all nodes with the given values for that field" in {
    val filter = """(where: { name: { in: ["Bernd", "Paul"] }})"""

    userUniques(filter) should be(Vector(1, 2))
  }

  "Using notIn" should "return all nodes not with the given values for that field" in {
    val filter = """(where: { name: { notIn: ["Bernd", "Paul"] }})"""

    userUniques(filter) should be(Vector(3, 4))
  }

  "Using notIn with null" should "return all nodes with not null values for that field" in {
    val filter = """(where: { name: { notIn: null }})"""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
  }

  "Relation Null filter" should "work" in {
    val filter = "(where: { ride: { is: null }})"

    userUniques(filter) should be(Vector(4))
  }

  "AND filter" should "work" in {
    val filter = """(where: {AND:[{unique: { gt: 2 }},{ name: { startsWith: "P" }}]})"""

    userUniques(filter) should be(Vector())
  }

  "Empty AND filter" should "work" in {
    val filter = """(where: {AND:[]})"""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
  }

  "OR filter" should "work" taggedAs (IgnoreMongo) in {
    val filter = """(where: {OR:[{unique: { gt: 2 }},{ name: { startsWith: "P" }}]})"""

    userUniques(filter) should be(Vector(1, 3, 4))
  }

  "Empty OR filter" should "work" taggedAs (IgnoreMongo) in {
    val filter = """(where: {OR:[]})"""

    userUniques(filter) should be(Vector())
  }

  "Empty NOT filter" should "work" taggedAs (IgnoreMongo) in {
    val filter = """(where: {NOT:[]})"""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
  }

  "NOT filter" should "work" taggedAs (IgnoreMongo) in {
    val filter = """(where: {NOT:{ name: { startsWith: "P" }}})"""

    userUniques(filter) should be(Vector(2, 3, 4))
  }

  "NOT filter" should "work as list" taggedAs (IgnoreMongo) in {
    val filter = """(where: { NOT:[{ name: { contains: "e" }},{unique: { equals: 1 }}]})"""

    userUniques(filter) should be(Vector(4))
  }

  "Nested filter" should "work" in {
    val filter = """(where: {ride: { is: { brand: { startsWith: "P" }}}})"""

    userUniques(filter) should be(Vector(1))
  }

  "Starts with filter" should "work" in {
    val filter = """(where: { name: { startsWith: "P"}})"""

    userUniques(filter) should be(Vector(1))
  }

  "Contains filter" should "work" in {
    val filter = """(where: { name: { contains: "n" }})"""

    userUniques(filter) should be(Vector(2, 4))
  }

  "Greater than filter" should "work with floats" in {
    val filter = """(where: {size: { gt: 100.500000000001 }})"""

    lotUniques(filter) should be(Vector(1))
  }

  "Inverted filters with null" should "work for optional fields" in {
    val filter = """(where: { name: { not: null }})"""

    userUniques(filter) should be(Vector(1, 2, 3, 4))
  }

  "Inverted filters with null" should "not work for required fields" in {
    val filter = """(where: { unique: { not: null }})"""

    server.queryThatMustFail(
      s"{ users $filter{ unique } }",
      project,
      errorCode = 2009,
      errorContains = "`Query.users.where.UserWhereInput.unique.IntFilter.not`: A value is required but not set."
    )
  }

  def userUniques(filter: String)    = server.query(s"{ users $filter{ unique } }", project).pathAsSeq("data.users").map(_.pathAsLong("unique")).toVector
  def vehicleUniques(filter: String) = server.query(s"{ vehicles $filter{ unique } }", project).pathAsSeq("data.vehicles").map(_.pathAsLong("unique")).toVector
  def lotUniques(filter: String) =
    server.query(s"{ parkingLots $filter{ unique } }", project).pathAsSeq("data.parkingLots").map(_.pathAsLong("unique")).toVector

  def populate: Unit = {
    server.query(
      s"""mutation createUser{createUser(
         |  data: {
         |    name: "Paul",
         |    unique:1,
         |    ride: {create: {brand: "Porsche",unique:1,parked: true}}
         |})
         |{id}
         |}
      """.stripMargin,
      project
    )

    server.query(
      s"""mutation createUser{createUser(
         |  data: {
         |    name: "Bernd",
         |    unique:2,
         |    ride: {create: {brand: "BMW",unique:2,parked: false}}
         |})
         |{id}
         |}
      """.stripMargin,
      project
    )

    server.query(
      s"""mutation createUser{createUser(
         |  data: {
         |    name: "Michael",
         |    unique:3,
         |    ride: {create: {brand: "Mercedes",unique:3,parked: true}}
         |})
         |{id}
         |}
      """.stripMargin,
      project
    )

    server.query(
      s"""mutation createUser{createUser(
         |  data: {
         |    name: "John",
         |    unique:4
         |})
         |{id}
         |}
      """.stripMargin,
      project
    )

    server.query(
      s"""mutation createParkingLot{createParkingLot(
         |  data: {
         |    area: "PrenzlBerg",
         |    unique:1,
         |    capacity: 12,
         |    size: 300.5
         |})
         |{id}
         |}
      """.stripMargin,
      project
    )

    server.query(
      s"""mutation createParkingLot{createParkingLot(
         |  data: {
         |    area: "Moabit",
         |    unique:2,
         |    capacity: 34,
         |    size: 100.5
         |})
         |{id}
         |}
      """.stripMargin,
      project
    )

  }
}
