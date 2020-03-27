package util

import play.api.libs.json.JsValue

trait SchemaBaseV11 extends PlayJsonExtensions {

  //Datamodel
  val simpleId = "id            String    @id @default(cuid())"
  val compoundId =
    """id_1          String        @default(cuid())
                        id_2          String        @default(cuid())
                        @@id([id_1, id_2])"""
  val noId = ""

  val idOptions = Vector(simpleId, compoundId, noId)

  // FIXME: this must go
  val idReference = "@relation(references: [id])"
  val noRef       = ""

  trait RelationReference {
    def apply(rf: RelationField): String
  }
  object RelationReference {
    def apply(fn: RelationField => String): RelationReference = new RelationReference {
      override def apply(rf: RelationField) = fn(rf)
    }
  }

  val simpleChildIdReference = RelationReference { rf =>
    if (rf.isList) {
      s"@relation(references: [id])"
    } else {
      s"@relation(fields: [childId], references: [id]) \n childId String${rf.optionalSuffix}"
    }
  }

  val simpleParentIdReference = RelationReference { rf =>
    if (rf.isList) {
      s"@relation(references: [id])"
    } else {
      s"@relation(fields: [parentId], references: [id]) \n parentId String${rf.optionalSuffix}"
    }
  }

  val compoundParentIdReference = RelationReference { rf =>
    //"@relation(references: [id_1, id_2]) @map([\"parent_id_1\", \"parent_id_2\"])"
    if (rf.isList) {
      s"@relation(references: [id_1, id_2])"
    } else {
      s"@relation(fields: [parent_id_1, parent_id_2], references: [id_1, id_2]) \n parent_id_1 String${rf.optionalSuffix}\n parent_id_2 String${rf.optionalSuffix}"
    }
  }
  val compoundChildIdReference = RelationReference { rf =>
    //"@relation(references: [id_1, id_2]) @map([\"child_id_1\", \"child_id_2\"])"
    if (rf.isList) {
      s"@relation(references: [id_1, id_2])"
    } else {
      s"@relation(fields: [child_id_1, child_id_2], references: [id_1, id_2])\n child_id_1 String${rf.optionalSuffix}\n child_id_2 String${rf.optionalSuffix}"
    }
  }

  val pReference = RelationReference { rf =>
    if (rf.isList) {
      s"@relation(references: [p])"
    } else {
      s"@relation(fields: [parentRef], references: [p]) \n parentRef String${rf.optionalSuffix}"
    }
  }
  val compoundPReference = RelationReference { rf =>
    //"@relation(references: [p_1, p_2]) @map([\"parent_p_1\", \"parent_p_2\"])"
    if (rf.isList) {
      s"@relation(references: [p_1, p_2])"
    } else {
      s"@relation(fields: [parent_p_1, parent_p_2], references: [p_1, p_2])\n parent_p_1 String${rf.optionalSuffix}\n parent_p_2 String${rf.optionalSuffix}"
    }
  }
  val cReference = RelationReference { rf =>
    if (rf.isList) {
      s"@relation(references: [c])"
    } else {
      s"@relation(fields:[parent_c], references: [c]) \nparent_c String${rf.optionalSuffix}"
    }
  }
  val compoundCReference = RelationReference { rf =>
    //"@relation(references: [c_1, c_2]) @map([\"child_c_1\", \"child_c_2\"])"
    if (rf.isList) {
      s"@relation(references: [c_1, c_2])"
    } else {
      s"@relation(fields: [child_c_1, child_c_2], references: [c_1, c_2])\n child_c_1 String${rf.optionalSuffix}\n child_c_2 String${rf.optionalSuffix}"
    }
  }

  def commonParentReferences(rf: RelationField): Vector[String] = Vector(pReference(rf), compoundPReference(rf))
  def commonChildReferences(rf: RelationField): Vector[String]  = Vector(cReference(rf), compoundCReference(rf))

  // parse functions

  def parseMultiCompound(fields: Vector[String], argName: String)(json: JsValue, path: String): Vector[String] = {
    json.pathAsJsArray(path).value.map(json => parseCompoundIdentifier(fields, argName)(json, "")).toVector
  }

  def parseMulti(field: String)(json: JsValue, path: String): Vector[String] = {
    json.pathAsJsArray(path).value.map(json => parseIdentifer(field)(json, "")).toVector
  }

  def parseCompoundIdentifier(fields: Vector[String], argName: String)(json: JsValue, path: String): String = {
    val pathPrefix  = if (path == "") "" else path + "."
    val fieldValues = fields.map(f => json.pathAsJsValue(pathPrefix + f))
    val arguments   = fields.zip(fieldValues).map { case (name, value) => s"""$name: $value""" }.mkString(",")

    s"""
       |{
       |  $argName: {
       |    $arguments
       |  }
       |}
       """.stripMargin
  }

  def parseIdentifer(field: String)(json: JsValue, path: String): String = {
    val finalPath = if (path == "") field else path + "." + field
    val value     = json.pathAsJsValue(finalPath).toString()
    s"{ $field: $value }"
  }

  sealed trait RelationField {
    def field: String
    def isList: Boolean        = false
    def optionalSuffix: String = if (field.endsWith("?")) "?" else ""
  }
  final case object ParentList extends RelationField {
    override def field: String   = "parentsOpt   Parent[]"
    override def isList: Boolean = true
  }
  final case object ChildList extends RelationField {
    override def field: String   = "childrenOpt  Child[]"
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

  def schemaWithRelation(onParent: RelationField, onChild: RelationField, withoutParams: Boolean = false) = {
    //Query Params
    val idParams = QueryParams(
      selection = "id",
      where = parseIdentifer("id"),
      whereMulti = parseMulti("id")
    )

    val compoundIdParams = {
      val fields  = Vector("id_1", "id_2")
      val argName = "id_1_id_2"
      QueryParams(
        selection = "id_1 , id_2",
        where = parseCompoundIdentifier(fields, argName),
        whereMulti = parseMultiCompound(fields, argName)
      )
    }

    val parentUniqueParams = Vector(
      QueryParams(
        selection = "p",
        where = parseIdentifer("p"),
        whereMulti = parseMulti("p")
      ), {
        val fields  = Vector("p_1", "p_2")
        val argName = "p_1_p_2"
        QueryParams(
          selection = "p_1, p_2",
          where = parseCompoundIdentifier(fields, argName),
          whereMulti = parseMultiCompound(fields, argName)
        )
      }
    )

    val childUniqueParams = Vector(
      QueryParams(
        selection = "c",
        where = parseIdentifer("c"),
        whereMulti = parseMulti("c")
      ), {
        val fields  = Vector("c_1", "c_2")
        val argName = "c_1_c_2"
        QueryParams(
          selection = "c_1, c_2",
          where = parseCompoundIdentifier(fields, argName),
          whereMulti = parseMultiCompound(fields, argName)
        )
      }
    )

    val simple = sys.env.getOrElse("TEST_MODE", "simple") == "simple"

    val datamodelsWithParams = for (parentId <- idOptions;
                                    childId <- idOptions;

                                    // Based on Id and relation fields
                                    childReference  <- childReferences(simple, parentId, onParent, onChild);
                                    parentReference <- parentReferences(simple, childId, childReference, onParent, onChild);

                                    // Only based on id
                                    parentParams <- if (withoutParams) {
                                                     Vector(idParams)
                                                   } else {
                                                     parentId match {
                                                       case `simpleId`   => parentUniqueParams :+ idParams
                                                       case `compoundId` => parentUniqueParams :+ compoundIdParams
                                                       case `noId`       => parentUniqueParams
                                                     }
                                                   };

                                    childParams <- if (withoutParams) {
                                                    Vector(idParams)
                                                  } else {
                                                    childId match {
                                                      case `simpleId`   => childUniqueParams :+ idParams
                                                      case `compoundId` => childUniqueParams :+ compoundIdParams
                                                      case `noId`       => childUniqueParams
                                                    }
                                                  })
      yield {
        val datamodel =
          s"""
                model Parent {
                    p             String    @unique
                    p_1           String?
                    p_2           String?
                    ${onParent.field}         $parentReference
                    non_unique    String?
                    $parentId

                    @@unique([p_1, p_2])
                }

                model Child {
                    c             String    @unique
                    c_1           String?
                    c_2           String?
                    ${onChild.field}          $childReference
                    non_unique    String?
                    $childId

                    @@unique([c_1, c_2])
                }
    """

        TestAbstraction(datamodel, parentParams, childParams)
      }

    AbstractTestDataModels(mongo = datamodelsWithParams, sql = datamodelsWithParams)
  }

  def childReferences(simple: Boolean, parentId: String, onParent: RelationField, onChild: RelationField): Vector[String] = {
    if (simple) {
      simpleChildReferences(parentId, onParent, onChild)
    } else {
      fullChildReferences(parentId, onParent, onChild)
    }
  }

  def simpleChildReferences(parentId: String, onParent: RelationField, onChild: RelationField): Vector[String] = {
    parentId match {
      case _ if onChild.isList && !onParent.isList => Vector(noRef)
      case `simpleId`                              => Vector(simpleParentIdReference(onChild))
      case `compoundId`                            => Vector(compoundParentIdReference(onChild))
      case `noId`                                  => Vector(pReference(onChild))
      case _                                       => ???
    }
  }

  def fullChildReferences(parentId: String, onParent: RelationField, onChild: RelationField): Vector[String] = {
    val isManyToMany = onParent.isList && onChild.isList

    if (!isManyToMany) {
      parentId match {
        case _ if onChild.isList && !onParent.isList => Vector(`noRef`)
        case `simpleId`                              => simpleParentIdReference(onChild) +: commonParentReferences(onChild)
        case `compoundId`                            => compoundParentIdReference(onChild) +: commonParentReferences(onChild)
        case _                                       => commonParentReferences(onChild)
      }
    } else {
      parentId match {
        case `simpleId`   => Vector(simpleParentIdReference(onChild))
        case `compoundId` => Vector(compoundParentIdReference(onChild))
        case _            => Vector(pReference(onChild))
      }
    }
  }

  def parentReferences(simple: Boolean, childId: String, childReference: String, onParent: RelationField, onChild: RelationField): Vector[String] = {
    if (simple) {
      simpleParentReferences(childId, childReference, onParent, onChild)
    } else {
      fullParentReferences(childId, childReference, onParent, onChild)
    }
  }

  def simpleParentReferences(childId: String, childReference: String, onParent: RelationField, onChild: RelationField): Vector[String] = {
    val isManyToMany = onParent.isList && onChild.isList

    childId match {
      case _ if childReference != `noRef` && !isManyToMany => Vector(noRef)
      case `simpleId`                                      => Vector(simpleChildIdReference(onParent))
      case `compoundId`                                    => Vector(compoundChildIdReference(onParent))
      case `noId`                                          => Vector(cReference(onParent))
      case _                                               => ???
    }
  }

  def fullParentReferences(childId: String, childReference: String, onParent: RelationField, onChild: RelationField): Vector[String] = {
    val isManyToMany = onParent.isList && onChild.isList

    if (!isManyToMany) {
      (childId, childReference) match {
        case (_, _) if onParent.isList && !onChild.isList => Vector(`noRef`)
        case (`simpleId`, `noRef`)                        => simpleChildIdReference(onParent) +: commonChildReferences(onParent)
        case (`simpleId`, _) if onParent.isList && onChild.isList =>
          simpleChildIdReference(onParent) +: commonChildReferences(onParent) :+ noRef

        case (`simpleId`, _)         => Vector(noRef)
        case (`compoundId`, `noRef`) => compoundChildIdReference(onParent) +: commonChildReferences(onParent)
        case (`compoundId`, _) if onParent.isList && onChild.isList =>
          compoundChildIdReference(onParent) +: commonChildReferences(onParent) :+ noRef

        case (`compoundId`, _)                                => Vector(noRef)
        case (`noId`, `noRef`)                                => commonChildReferences(onParent)
        case (`noId`, _) if onParent.isList && onChild.isList => commonChildReferences(onParent) :+ noRef
        case (`noId`, _)                                      => Vector(noRef)
        case (_, _)                                           => Vector.empty
      }
    } else {
      childId match {
        case `simpleId`   => Vector(simpleChildIdReference(onParent))
        case `compoundId` => Vector(compoundChildIdReference(onParent))
        case _            => Vector(cReference(onParent))
      }
    };
  }

  //region NON EMBEDDED WITH @id

  val schemaP1optToC1opt = {
    val s1 = """
      model Parent {
          id       String  @id @default(cuid())
          p        String  @unique
          childOpt Child?  @relation(fields: [childId], references: [id])
          childId  String?
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
          parentOpt Parent? @relation(fields: [parentId],references: [id])
          parentId  String?
      }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1, s2))
  }

  val schemaP1optToCM = {
    val s1 = """
    model Parent {
        id       String  @id @default(cuid())
        p        String  @unique
        childOpt Child?  @relation(fields: [childId], references: [id])
        childId  String?
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
        parentsOpt Parent[] @relation(fields: [parentIds], references: [id])
        parentIds  String[]
    }"""

    TestDataModels(mongo = Vector(s1, s2), sql = Vector(s1))
  }
}
