package util

trait SchemaBaseV11 {

  //Datamodel
  val simpleId   = "id            String    @id @default(cuid())"
  val compoundId = """id_1          String        @default(cuid())
                        id_2          String        @default(cuid())
                        @@id([id_1, id_2])"""
  val noId       = ""

  val idOptions = Vector(simpleId, compoundId, noId)

  val idReference               = "@relation(references: [id])"
  val noRef                     = ""
  val compoundParentIdReference = "@relation(references: [id_1, id_2]) @map([\"parent_id_1\", \"parent_id_2\"])"
  val compoundChildIdReference  = "@relation(references: [id_1, id_2]) @map([\"child_id_1\", \"child_id_2\"])"

  val pReference         = "@relation(references: [p])"
  val compoundPReference = "@relation(references: [p_1, p_2]) @map([\"parent_p_1\", \"parent_p_2\"])"
  val cReference         = "@relation(references: [c])"
  val compoundCReference = "@relation(references: [c_1, c_2]) @map([\"child_c_1\", \"child_c_2\"])"

  val commonParentReferences = Vector(pReference, compoundPReference, noRef)
  val commonChildReferences  = Vector(cReference, compoundCReference)

  //Query Params
  val idParams           = QueryParams("id", "id", ".id")
  val compoundIdParams   = QueryParams("id_1_id_2", "id_1 , id_2", "")
  val parentUniqueParams = Vector(QueryParams("p", "p", ".p"), QueryParams("p_1_p_2", "p_1, p_2", ""))
  val childUniqueParams  = Vector(QueryParams("c", "c", ".c"), QueryParams("c_1_c_2", "c_1, c_2", ""))

  sealed trait RelationField {
    def field: String
    def isList: Boolean = false
  }
  final case object ParentList extends RelationField {
    override def field: String = "parentsOpt   Parent[]"
    override def isList: Boolean = true
  }
  final case object ChildList extends RelationField {
    override def field: String = "childrenOpt  Child[]"
    override def isList: Boolean = true
  }

  final case object ParentOpt extends RelationField {
    override def field: String = "parentOpt     Parent?"
  }
  final case object ParentReq extends RelationField {
    override def field: String = "parentReq     Parent"
  }
  final case object ChildOpt extends RelationField {
    override def field: String = "childOpt      Child?"
  }
  final case object ChildReq extends RelationField {
    override def field: String = "childReq      Child"
  }

  def schemaWithRelation(onParent: RelationField, onChild: RelationField) = {

    val datamodelsWithParams = for (parentId <- idOptions;
                                    childId <- idOptions;
                                    //based on Id and relation
                                    childReferences <- parentId match {
                                                        case _ if onChild.isList && !onParent.isList => Vector(`noRef`)
                                                        case `simpleId`           => idReference +: commonParentReferences
                                                        case `compoundId`         => compoundParentIdReference +: commonParentReferences
                                                        case _                   => commonParentReferences
                                                      };
                                    parentReferences <- (childId, childReferences)  match {
                                                         case (_, _) if onParent.isList && !onChild.isList => Vector(`noRef`)
                                                         case (`simpleId`, `noRef`)     => idReference +: commonChildReferences
                                                         case (`simpleId`, _) if onParent.isList && onChild.isList => idReference +: commonChildReferences :+ noRef
                                                         case (`simpleId`, _)           => Vector(noRef)
                                                         case (`compoundId`, `noRef`)   => compoundChildIdReference +: commonChildReferences
                                                         case (`compoundId`, _) if onParent.isList && onChild.isList => compoundChildIdReference +: commonChildReferences :+ noRef
                                                         case (`compoundId`, _)         => Vector(noRef)
                                                         case (`noId`, `noRef`)         => commonChildReferences
                                                         case (`noId`, _) if onParent.isList && onChild.isList => commonChildReferences :+ noRef
                                                         case (`noId`, _)               => Vector(noRef)
                                                         case (_, _)                    => Vector.empty
                                                       };
                                    //only based on id
                                    parentParams <- parentId match {
                                                     case `simpleId`   => parentUniqueParams :+ idParams
                                                     case `compoundId` => parentUniqueParams :+ compoundIdParams
                                                     case `noId`       => parentUniqueParams
                                                   };
                                    childParams <- childId match {
                                                    case `simpleId`   => childUniqueParams :+ idParams
                                                    case `compoundId` => childUniqueParams :+ compoundIdParams
                                                    case `noId`       => parentUniqueParams
                                                  })
      yield {
        val datamodel =
          s"""
                model Parent {
                    p             String    @unique
                    p_1           String?
                    p_2           String?
                    ${onParent.field}         $parentReferences
                    non_unique    String?
                    $parentId

                    @@unique([p_1, p_2])
                }

                model Child {
                    c             String    @unique
                    c_1           String?
                    c_2           String?
                    ${onChild.field}          $childReferences
                    non_unique    String?
                    $childId

                    @@unique([c_1, c_2])
                }
    """

        TestAbstraction(datamodel, parentParams, childParams)
      }

    println(datamodelsWithParams.length)

    AbstractTestDataModels(mongo = datamodelsWithParams, sql = datamodelsWithParams)
  }

  //region NON EMBEDDED WITH @id

//  val schemaP1reqToC1reqWithId = {
//    val s1 = """
//    model Parent {
//        id            String    @id @default(cuid())
//        p             String    @unique
//        p_1           String?
//        p_2           String?
//        childReq      Child     @relation(references: [id])
//        non_unique    String?
//
//        @@unique([p_1, p_2])
//    }
//
//    model Child {
//        id            String    @id @default(cuid())
//        c             String    @unique
//        c_1           String?
//        c_2           String?
//        parentReq     Parent
//        non_unique    String?
//
//        @@unique([c_1, c_2])
//    }"""
//
//    val s2 = """
//    model Parent {
//        id            String    @id @default(cuid())
//        p             String    @unique
//        p_1           String?
//        p_2           String?
//        childReq      Child
//        non_unique    String?
//
//        @@unique([p_1, p_2])
//    }
//
//    model Child {
//        id            String    @id @default(cuid())
//        c             String    @unique
//        c_1           String?
//        c_2           String?
//        parentReq     Parent    @relation(references: [id])
//        non_unique    String?
//
//        @@unique([c_1, c_2])
//    }"""
//
//    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
//  }

  val schemaP1reqToC1req = {
    val s1 = """
    """

    TestDataModels(mongo = Vector(s1), sql = Vector(s1))
  }

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
