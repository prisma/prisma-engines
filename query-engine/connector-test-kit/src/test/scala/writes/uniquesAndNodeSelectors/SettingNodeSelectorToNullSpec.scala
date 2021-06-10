package writes.uniquesAndNodeSelectors

import org.scalatest.{FlatSpec, Matchers}
import util._

// RS: Ported
class SettingNodeSelectorToNullSpec extends FlatSpec with Matchers with ApiSpecBase {

    val project = ProjectDsl.fromString {
      "Setting a where value to null " should " work when there is no further nesting " in {
        """
        |model A {
        |  id  String  @id @default(cuid())
        |  b   String? @unique
        |  key String  @unique
        |}
      """
    }
    database.setup(project)

    server.query(
      """mutation a {
        |  createA(data: {
        |    b: "abc"
        |    key: "abc"
        |  }) {
        |    id
        |  }
        |}""",
      project
    )

    val res = server.query(
      """mutation b {
        |  updateA(
        |    where: { b: "abc" }
        |    data: {
        |      b: { set: null }
        |    }) {
        |    b
        |  }
        |}""",
      project
    )

    res.toString() should be("""{"data":{"updateA":{"b":null}}}""")
  }
}
