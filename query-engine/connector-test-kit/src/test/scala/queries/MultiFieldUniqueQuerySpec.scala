package queries

import org.scalatest.{FlatSpec, Matchers}
import play.api.libs.json.JsNull
import util._

class MultiFieldUniqueQuerySpec extends FlatSpec with Matchers with ApiSpecBase {
  "A simple multi-field-unique query" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id        String @id @default(cuid())
                                                |  FirstName String
                                                |  LastName  String
                                                |
                                                |  @@unique([FirstName, LastName])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    def createUser(firstName: String, lastName: String): String = {
      server
        .query(
          s"""mutation {
                  |  createUser(data: {FirstName: "$firstName", LastName: "$lastName"}) {
                  |    id
                  |  }
                  |}""".stripMargin,
          project
        )
        .pathAsString("data.createUser.id")
    }

    val userId = createUser("Hans", "Wurst")
    createUser("Matt", "Eagle")

    val result = server.query(
      s"""{
        |  user(where: {FirstName_LastName: {
        |    FirstName: "Hans"
        |    LastName: "Wurst"
        |  }}){
        |    id
        |  }
        |}""".stripMargin,
      project
    )

    result.pathAsString("data.user.id") should equal(userId)
  }

  "A simple multi-field-unique query on an aliased index" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id        String @id @default(cuid())
                                                |  FirstName String
                                                |  LastName  String
                                                |
                                                |  @@unique([FirstName, LastName], name: "full_name")
                                                |}
                                              """.stripMargin }
    database.setup(project)

    def createUser(firstName: String, lastName: String): String = {
      server
        .query(
          s"""mutation {
                  |  createUser(data: {FirstName: "$firstName", LastName: "$lastName"}) {
                  |    id
                  |  }
                  |}""".stripMargin,
          project
        )
        .pathAsString("data.createUser.id")
    }

    val userId = createUser("Hans", "Wurst")
    createUser("Matt", "Eagle")

    val result = server.query(
      s"""{
                                 |  user(where: {full_name: {
                                 |    FirstName: "Hans"
                                 |    LastName: "Wurst"
                                 |  }}){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project
    )

    result.pathAsString("data.user.id") should equal(userId)
  }

  "A simple multi-field-unique query on a heterogeneous index" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id        String @id @default(cuid())
                                                |  FirstName String
                                                |  LastName  String
                                                |  SSN       Int
                                                |
                                                |  @@unique([FirstName, LastName, SSN])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    def createUser(firstName: String, lastName: String, ssn: Int): String = {
      server
        .query(
          s"""mutation {
                  |  createUser(data: {FirstName: "$firstName", LastName: "$lastName", SSN: $ssn}) {
                  |    id
                  |  }
                  |}""".stripMargin,
          project
        )
        .pathAsString("data.createUser.id")
    }

    val userId = createUser("Hans", "Wurst", 123)
    createUser("Matt", "Eagle", 321)

    val result = server.query(
      s"""{
                                 |  user(where: {FirstName_LastName_SSN: {
                                 |    FirstName: "Hans"
                                 |    LastName: "Wurst"
                                 |    SSN: 123
                                 |  }}){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project
    )

    result.pathAsString("data.user.id") should equal(userId)
  }

  "A simple multi-field-unique query on a nonexistent user" should "return null" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id        String @id @default(cuid())
                                                |  FirstName String
                                                |  LastName  String
                                                |
                                                |  @@unique([FirstName, LastName])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val result = server.query(
      s"""{
                                 |  user(where: {FirstName_LastName: {
                                 |    FirstName: "Foo"
                                 |    LastName: "Bar"
                                 |  }}){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project
    )

    result.pathAsJsValue("data.user") should equal(JsNull)
  }

  "A simple multi-field-unique query with an incomplete where" should "fail" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id        String @id @default(cuid())
                                                |  FirstName String
                                                |  LastName  String
                                                |
                                                |  @@unique([FirstName, LastName])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    server.query(
      s"""{
                                 |  user(where: {FirstName_LastName: {
                                 |    FirstName: "Hans"
                                 |    LastName: "Wurst"
                                 |  }}){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project
    )

    server.queryThatMustFail(
      s"""{
                                 |  user(where: {FirstName_LastName: {
                                 |    FirstName: "Hans"
                                 |  }}){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project,
      1234
    )
  }

  "Querying a multi-field-unique schema with a ludicrous number of fields" should "succeed" taggedAs IgnoreMySql in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id        String @id @default(cuid())
                                                |  a String
                                                |  b String
                                                |  c String
                                                |  d String
                                                |  e String
                                                |  f String
                                                |  g String
                                                |  h String
                                                |  i String
                                                |  j String
                                                |  k String
                                                |  l String
                                                |  m String
                                                |  n String
                                                |  o String
                                                |  p String
                                                |  q String
                                                |  r String
                                                |  s String
                                                |  t String
                                                |  u String
                                                |  v String
                                                |  w String
                                                |  x String
                                                |  y String
                                                |  z String
                                                |  @@unique([a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val user = server.query(
      s"""mutation {
                                 |  createUser(data: {
                                 |      a: "test"
                                 |      b: "test"
                                 |      c: "test"
                                 |      d: "test"
                                 |      e: "test"
                                 |      f: "test"
                                 |      g: "test"
                                 |      h: "test"
                                 |      i: "test"
                                 |      j: "test"
                                 |      k: "test"
                                 |      l: "test"
                                 |      m: "test"
                                 |      n: "test"
                                 |      o: "test"
                                 |      p: "test"
                                 |      q: "test"
                                 |      r: "test"
                                 |      s: "test"
                                 |      t: "test"
                                 |      u: "test"
                                 |      v: "test"
                                 |      w: "test"
                                 |      x: "test"
                                 |      y: "test"
                                 |      z: "test"
                                 |  }){
                                 |    id
                                 |  }
                                 |}""".stripMargin,
      project
    )

    val id = user.pathAsString("data.createUser.id")

    val result = server.query(
      s"""{
                      |  user(where: {a_b_c_d_e_f_g_h_i_j_k_l_m_n_o_p_q_r_s_t_u_v_w_x_y_z: {
                      |    a: "test"
                      |    b: "test"
                      |    c: "test"
                      |    d: "test"
                      |    e: "test"
                      |    f: "test"
                      |    g: "test"
                      |    h: "test"
                      |    i: "test"
                      |    j: "test"
                      |    k: "test"
                      |    l: "test"
                      |    m: "test"
                      |    n: "test"
                      |    o: "test"
                      |    p: "test"
                      |    q: "test"
                      |    r: "test"
                      |    s: "test"
                      |    t: "test"
                      |    u: "test"
                      |    v: "test"
                      |    w: "test"
                      |    x: "test"
                      |    y: "test"
                      |    z: "test"
                      |  }}){
                      |    id
                      |  }
                      |}""".stripMargin,
      project
    )

    result.pathAsString("data.user.id") should equal(id)
  }

  "Not using the compound unique as the UniqueInput" should "be allowed" in {
    val project = SchemaDsl.fromStringV11() {
      """                model Parent {
        |                    p_1           String?
        |                    p_2           String?
        |                    id            String    @id @default(cuid())
        |
        |                    @@unique([p_1, p_2])
        |                }
        |
        |           """.stripMargin
    }

    database.setup(project)

    server.query("""query{parent(where:{id: "1"}){id}}""", project)

  }
}
