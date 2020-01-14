package util

trait SchemaBaseV11 {

  //todo make this super generic over the datamodel

  val schemaP1reqToC1req = {

    // todo allow to pass in flags to enable/disable certain combinations ??

//    vectors of strings
//    for comprehension over the vectors
//    generate datamodel,
//    then loop further over the options for the query params
//    return case class containing all
//    pass that one to the tests





    val parent_id =  "id            String    @id @default(cuid())"

    //val parent_id =    """id_1          String    @id @default(cuid())
    //                      id_2          String    @id @default(cuid())
    //                      @@unique([id_1, id_2])"""

    //val parent_id = ""

    val child_id =    "id            String    @id @default(cuid())"

    //val child_id =    """id_1          String    @id @default(cuid())
    //                     id_2          String    @id @default(cuid())
    //                     @@unique([id_1, id_2])"""

    //val child_id =    ""



     val (relation_parent, relation_child) =      ("@relation(references: [id])", "")
    // val (relation_parent, relation_child) =     ("", "@relation(references: [id])")

    // val (relation_parent, relation_child) =     ("@relation(references: [id_1, id_2]) @map(["child_id_1", "child_id_2"])", "")
    // val (relation_parent, relation_child) =     ("", "@relation(references: [id_1, id_2]) @map(["parent_id_1", "parent_id_2"])")

    // val (relation_parent, relation_child) =     ("@relation(references: [c])", "")
    // val (relation_parent, relation_child) =     ("", "@relation(references: [p])")

    // val (relation_parent, relation_child) =     ("@relation(references: [c_1, c_2]) @map(["child_c_1", "child_c_2"])", "")
    // val (relation_parent, relation_child) =     ("", "@relation(references: [p_1, p_2]) @map(["parent_p_1", "parent_p_2"])")


    val s1 = s"""
    model Parent {
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child     $relation_parent
        non_unique    String?
        $parent_id

        @@unique([p_1, p_2])
    }

    model Child {
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent    $relation_child
        non_unique    String?
        $child_id

        @@unique([c_1, c_2])
    }"""


//    Test Case Class
//    Datamodel
//    parentIdentifierName
//    parentReturnValue
//    parentReturnValueParse
//    childIdentifierName
//    childReturnValue
//    childReturnValueParse

//    println(s1)
    //todo generate different data models here,
    //pass back the necessary placeholders for the operation together with the datamodel

    TestDataModels(mongo = Vector(s1), sql = Vector(s1))
  }



  //region NON EMBEDDED WITH @id

  val schemaP1reqToC1reqWithId = {
    val s1 = """
    model Parent {
        id            String    @id @default(cuid())
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child     @relation(references: [id])
        non_unique    String?

        @@unique([p_1, p_2])
    }

    model Child {
        id            String    @id @default(cuid())
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent
        non_unique    String?

        @@unique([c_1, c_2])
    }"""

    val s2 = """
    model Parent {
        id            String    @id @default(cuid())
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child
        non_unique    String?

        @@unique([p_1, p_2])
    }

    model Child {
        id            String    @id @default(cuid())
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent    @relation(references: [id])
        non_unique    String?

        @@unique([c_1, c_2])
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }

  val schemaP1reqToC1reqWithCompoundId = {
    val s1 = """
    model Parent {
        id_1          String    @default(cuid())
        id_2          String    @default(cuid())
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child     @relation(references: [id_2, id_2]) @map("child_id_1", "child_id_2")
        non_unique    String?

        @@id([id_1, id_2])
        @@unique([p_1, p_2])
    }

    model Child {
        id_1          String    @default(cuid())
        id_2          String    @default(cuid())
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent
        non_unique    String?

        @@id([id_1, id_2])
        @@unique([c_1, c_2])
    }"""

    val s2 = """
     model Parent {
        id_1          String    @default(cuid())
        id_2          String    @default(cuid())
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child
        non_unique    String?

        @@id([id_1, id_2])
        @@unique([p_1, p_2])
    }

    model Child {
        id_1          String    @default(cuid())
        id_2          String    @default(cuid())
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent    @relation(references: [id_2, id_2]) @map("parent_id_1", "parent_id_2")
        non_unique    String?

        @@id([id_1, id_2])
        @@unique([c_1, c_2])
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }

  val schemaP1reqToC1reqWithoutId = {
    val s1 = """
    model Parent {
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child     @relation(references: [c])
        non_unique    String?

        @@unique([p_1, p_2])
    }

    model Child {
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent
        non_unique    String?

        @@unique([c_1, c_2])
    }"""

    val s2 = """
    model Parent {
        p             String    @unique
        p_1           String?
        p_2           String?
        childReq      Child
        non_unique    String?

        @@unique([p_1, p_2])
    }

    model Child {
        c             String    @unique
        c_1           String?
        c_2           String?
        parentReq     Parent    @relation(references: [p_1, p_2]) @map(["parent_p_1", "parent_p_2"])
        non_unique    String?

        @@unique([c_1, c_2])
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }


  // todo

  val schemaP1optToC1req = {
    val s1 = """
    model Parent {
        id       String @id @default(cuid())
        p        String @unique
        childOpt Child? @relation(references: [id])
    }

    model Child {
        id        String @id @default(cuid())
        c         String @unique
        parentReq Parent
    }"""

    val s2 = """
    model Parent {
        id       String @id @default(cuid())
        p        String @unique
        childOpt Child?
    }

    model Child {
        id        String @id @default(cuid())
        c         String @unique
        parentReq Parent @relation(references: [id])
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }

  val schemaP1optToC1opt = {
    val s1 = """
      model Parent {
          id       String @id @default(cuid())
          p        String @unique
          childOpt Child? @relation(references: [id])
      }

      model Child {
          id        String @id @default(cuid())
          c         String @unique
          parentOpt Parent?
      }"""

    val s2 = """
      model Parent {
          id       String @id @default(cuid())
          p        String @unique
          childOpt Child?
      }

      model Child {
          id        String  @id @default(cuid())
          c         String  @unique
          parentOpt Parent? @relation(references: [id])
      }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }

  val schemaP1reqToC1opt = {
    val s1 = """
      model Parent {
          id       String @id @default(cuid())
          p        String @unique
          childReq Child  @relation(references: [id])
      }

      model Child {
          id        String  @id @default(cuid())
          c         String  @unique
          parentOpt Parent?
        }"""

    val s2 = """
      model Parent {
          id       String @id @default(cuid())
          p        String @unique
          childReq Child
      }

      model Child {
          id        String  @id @default(cuid())
          c         String  @unique
          parentOpt Parent? @relation(references: [id])
      }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }

  val schemaPMToC1req = {
    val s1 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[] @relation(references: [id])
    }

    model Child {
        id        String @id @default(cuid())
        c         String @unique
        parentReq Parent
        test      String?
    }"""

    val s2 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[]
    }

    model Child {
        id        String  @id @default(cuid())
        c         String  @unique
        parentReq Parent  @relation(references: [id])
        test      String?
    }"""

    val s3 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[]
    }

    model Child {
        id        String @id @default(cuid())
        c         String @unique
        parentReq Parent
        test      String?
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s2, s3))
  }

  val schemaPMToC1opt = {
    val s1 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[] @relation(references: [id])
    }

    model Child {
        id        String @id @default(cuid())
        c         String @unique
        parentOpt Parent?
        test      String
    }"""

    val s2 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[]
    }

    model Child {
        id        String  @id @default(cuid())
        c         String  @unique
        parentOpt Parent? @relation(references: [id])
        test      String?
    }"""

    val s3 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[]
    }

    model Child {
        id        String  @id @default(cuid())
        c         String  @unique
        parentOpt Parent?
        test      String?
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s2, s3))
  }

  val schemaP1reqToCM = {
    val s1 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String  @unique
        childReq Child   @relation(references: [id])
    }

    model Child {
        id         String  @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[]
    }"""

    val s2 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String  @unique
        childReq Child
    }

    model Child {
        id         String   @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[] @relation(references: [id])
    }"""

    val s3 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String  @unique
        childReq Child
    }

    model Child {
        id         String  @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[]
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s3))
  }

  val schemaP1optToCM = {
    val s1 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String  @unique
        childOpt Child?  @relation(references: [id])
    }

    model Child {
        id         String   @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[]
    }"""

    val s2 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String? @unique
        childOpt Child?
    }

    model Child {
        id         String   @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[] @relation(references: [id])
    }"""

    val s3 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String  @unique
        childOpt Child?
    }

    model Child {
        id         String   @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[]
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s3))
  }

  val schemaPMToCM = {
    val s1 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String? @unique
        childrenOpt Child[] @relation(references: [id])
    }

    model Child {
        id         String   @id @default(cuid())
        c          String   @unique
        parentsOpt Parent[]
        test       String?
    }"""

    val s2 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[]
    }

    model Child {
        id         String   @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[] @relation(references: [id])
        test       String?
    }"""

    val s3 = """
    model Parent {
        id          String  @id @default(cuid())
        p           String  @unique
        childrenOpt Child[]
    }

    model Child {
        id         String   @id @default(cuid())
        c          String  @unique
        parentsOpt Parent[]
        test       String?
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s3))
  }

  //endregion

  //region EMBEDDED

  val embeddedP1req = """model Parent {
                            id       String  @id @default(cuid())
                            p        String  @unique
                            childReq Child
                        }

                        model Child @embedded {
                            c String
                        }"""

  val embeddedP1opt = """model Parent {
                            id       String  @id @default(cuid())
                            p        String  @unique
                            childOpt Child?
                        }

                        model Child @embedded {
                            c String
                        }"""

  val embeddedPM = """model Parent {
                            id          String  @id @default(cuid())
                            p           String  @unique
                            childrenOpt Child[]
                        }

                        model Child @embedded{
                            id   String @id @default(cuid())
                            c    String
                            test String?
                        }"""

  //endregion

  //region EMBEDDED TO NON-EMBEDDED
  val embedddedToJoinFriendReq = """
                            |model Parent {
                            |    id       String  @id @default(cuid())
                            |    p        String? @unique
                            |    children Child[]
                            |}
                            |
                            |model Child @embedded {
                            |    id        String  @id @default(cuid())
                            |    c         String?
                            |    friendReq Friend
                            |}
                            |
                            |model Friend{
                            |    id String @id @default(cuid())
                            |    f  String @unique
                            |}"""

  val embedddedToJoinFriendOpt = """
                               |model Parent {
                               |    id       String  @id @default(cuid())
                               |    p        String? @unique
                               |    children Child[]
                               |}
                               |
                               |model Child @embedded {
                               |    id        String  @id @default(cuid())
                               |    c         String?
                               |    friendOpt Friend?
                               |}
                               |
                               |model Friend{
                               |    id String @id @default(cuid())
                               |    f  String @unique
                               |}"""

  val embedddedToJoinFriendsOpt = """
                        |model Parent {
                        |    id       String  @id @default(cuid())
                        |    p        String? @unique
                        |    children Child[]
                        |}
                        |
                        |model Child @embedded {
                        |    id         String  @id @default(cuid())
                        |    c          String?
                        |    friendsOpt Friend[]
                        |}
                        |
                        |model Friend{
                        |    id   String  @id @default(cuid())
                        |    f    String? @unique
                        |    test String?
                        |}"""

  //endregion
}

