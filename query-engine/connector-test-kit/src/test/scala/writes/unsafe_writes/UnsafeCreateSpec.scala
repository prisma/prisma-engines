package writes.dataTypes.bytes

import org.scalatest.{FlatSpec, Matchers}
import util._

class UnsafeCreateSpec extends FlatSpec with Matchers with ApiSpecBase {
  // CREATE
  // 1) Test with 2 relations inlined (one opt, one req), one not inlined. Make it multi-field!
  // -> required relations need to be written.
  // -> Optional can be optional.
  // -> Other relation is still there that is not inlined.
  // 2) Auto inc ids can be provided, so id is an OPTIONAL input.
  // 3) Test empty create input and when it's present and when it's not. Can have weird impact on existing queries.
  // 4) Test with default on relation scalar?
  // 5)

  "Uncehcked creates" should "allow writing inlined relation scalars" in {
    val project = ProjectDsl.fromString {
      // test optional relation with one field optional one field required?
      """|model ModelA {
         |  id     Int  @id
         |  b_id_1 Int
         |  b_id_2 Int
         |  c_id_1 Int?
         |  c_id_2 Int?
         |
         |  b B @relation(fields: [b_id_1, b_id_2], references: [uniq_1, uniq_2])
         |  c C @relation(fields: [c_id_1, c_id_2], references: [uniq_1, uniq_2])
         |}
         |
         |model ModelB {
         |  uniq_1    Int
         |  uniq_2    Int
         |
         |  @@unique([uniq_1, uniq_2])
         |}
         |
         |model ModelC {
         |  uniq_1    Int
         |  uniq_2    Int
         |
         |  @@unique([uniq_1, uniq_2])
         |}
         """
    }

    database.setup(project)

  }
}
