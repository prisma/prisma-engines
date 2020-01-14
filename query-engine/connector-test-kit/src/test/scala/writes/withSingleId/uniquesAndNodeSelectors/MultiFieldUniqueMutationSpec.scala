package writes.withSingleId.uniquesAndNodeSelectors

import org.scalatest.{FlatSpec, Matchers}
import util._

class MultiFieldUniqueMutationSpec extends FlatSpec with Matchers with ApiSpecBase {

  // CONNECTS //

  "A nested connect on a one-to-one relation with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                  |  id   String @id @default(cuid())
                                                  |  name String
                                                  |  blog Blog?
                                                  |}
                                                  |
                                                  |model Blog {
                                                  |  id       String @id @default(cuid())
                                                  |  title    String
                                                  |  category String
                                                  |  author   User?
                                                  |
                                                  |  @@unique([title, category])
                                                  |}
                                                """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
             |  createUser(data: {
             |    name: "Thomas the Tank Engine"
             |  }) {
             |    id
             |  }
             |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val blogId = server
      .query(
        s"""mutation {
           |  createBlog(data: {
           |    title: "Thomas has seen it all. Thomas is leaving."
           |    category: "Horror"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createBlog.id")

    val result = server.query(
      s"""mutation {
           |  updateUser(where: {
           |    id: "$userId"
           |  }
           |  data: {
           |    blog: {
           |      connect: {
           |        title_category: {
           |          title: "Thomas has seen it all. Thomas is leaving."
           |          category: "Horror"
           |        }
           |      }
           |  }}){
           |    blog {
           |      id
           |    }
           |  }
           |}""".stripMargin,
      project
    )

    result.pathAsString("data.updateUser.blog.id") should equal(blogId)
  }

  "A nested connect on a one-to-many relation with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id    String @id @default(cuid())
                                                |  name  String
                                                |  blogs Blog[]
                                                |}
                                                |
                                                |model Blog {
                                                |  id       String @id @default(cuid())
                                                |  title    String
                                                |  category String
                                                |  author   User?
                                                |
                                                |  @@unique([title, category])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    name: "Thomas the Tank Engine"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val blogId = server
      .query(
        s"""mutation {
           |  createBlog(data: {
           |    title: "Thomas has seen it all. Thomas is leaving."
           |    category: "Horror"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createBlog.id")

    val result = server.query(
      s"""mutation {
         |  updateUser(where: {
         |    id: "$userId"
         |  }
         |  data: {
         |    blogs: {
         |      connect: {
         |        title_category: {
         |          title: "Thomas has seen it all. Thomas is leaving."
         |          category: "Horror"
         |        }
         |      }
         |  }}){
         |    blogs {
         |      id
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsJsArray("data.updateUser.blogs").value.length should equal(1)
  }

  // DISCONNECTS //

  "A nested disconnect with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id    String @id @default(cuid())
                                                |  name  String
                                                |  blogs Blog[]
                                                |}
                                                |
                                                |model Blog {
                                                |  id       String @id @default(cuid())
                                                |  title    String
                                                |  category String
                                                |  author   User?
                                                |
                                                |  @@unique([title, category])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    name: "Sly Marbo"
           |    blogs: {
           |      create: [{
           |        title: "AAAAAAAAAAA!"
           |        category: "Drama"
           |      },
           |      {
           |        title: "The Secret of AAAAAAAAAAA!"
           |        category: "Drama"
           |      }]
           |    }
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val result = server.query(
      s"""mutation {
         |  updateUser(where: {
         |    id: "$userId"
         |  }
         |  data: {
         |    blogs: {
         |      disconnect: {
         |        title_category: {
         |          title: "AAAAAAAAAAA!"
         |          category: "Drama"
         |        }
         |      }
         |  }}) {
         |    blogs {
         |      id
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsJsArray("data.updateUser.blogs").value.length should equal(1)
  }

  // UPDATES //

  "An update with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id          String @id @default(cuid())
                                                |  first_name  String
                                                |  last_name   String
                                                |  
                                                |  @@unique([first_name, last_name])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    first_name: "Justin"
           |    last_name: "Case"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val result = server.query(
      s"""mutation {
         |  updateUser(where: {
         |    first_name_last_name: {
         |      first_name: "Justin"
         |      last_name: "Case"
         |    }
         |  }
         |  data: {
         |    first_name: "Worst"
         |  }) {
         |    first_name
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsString("data.updateUser.first_name") should equal("Worst")
  }

  "A nested update with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id    String @id @default(cuid())
                                                |  name  String
                                                |  blogs Blog[]
                                                |}
                                                |
                                                |model Blog {
                                                |  id        String @id @default(cuid())
                                                |  title     String
                                                |  category  String
                                                |  published Boolean
                                                |  author    User?
                                                |
                                                |  @@unique([title, category])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    name: "King Arthur"
           |    blogs: {
           |      create: [{
           |        title: "A Practical Guide to the Monster of Caerbannog"
           |        category: "Education"
           |        published: false
           |      }]
           |    }
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val result = server.query(
      s"""mutation {
         |  updateUser(where: {
         |    id: "$userId"
         |  }
         |  data: {
         |    blogs: {
         |      update: {
         |        where: {
         |          title_category: {
         |            title: "A Practical Guide to the Monster of Caerbannog"
         |            category: "Education"
         |          }
         |        },
         |        data: {
         |          published: true
         |        }
         |      }
         |  }}) {
         |    blogs {
         |      published
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsJsArray("data.updateUser.blogs").value.head.pathAsBool("published") should equal(true)
  }

  // DELETES //

  "A delete with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id          String @id @default(cuid())
                                                |  first_name  String
                                                |  last_name   String
                                                |  
                                                |  @@unique([first_name, last_name])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    first_name: "Darth"
           |    last_name: "Llama"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val result = server.query(
      s"""mutation {
         |  deleteUser(where: {
         |    first_name_last_name: {
         |      first_name: "Darth"
         |      last_name: "Llama"
         |    }
         |  }) {
         |    id
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsString("data.deleteUser.id") should equal(userId)
  }

  "A nested delete with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id    String @id @default(cuid())
                                                |  name  String
                                                |  blogs Blog[]
                                                |}
                                                |
                                                |model Blog {
                                                |  id        String @id @default(cuid())
                                                |  title     String
                                                |  category  String
                                                |  author    User?
                                                |
                                                |  @@unique([title, category])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    name: "Matt Eagle"
           |    blogs: {
           |      create: [{
           |        title: "The Perfect German 'Mettigel'"
           |        category: "Cooking"
           |      }]
           |    }
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val result = server.query(
      s"""mutation {
         |  updateUser(where: {
         |    id: "$userId"
         |  }
         |  data: {
         |    blogs: {
         |      delete: {
         |          title_category: {
         |            title: "The Perfect German 'Mettigel'"
         |            category: "Cooking"
         |          }
         |      }
         |  }}) {
         |    blogs {
         |      id
         |    }
         |  }
         |}""".stripMargin,
      project
    )

    result.pathAsJsArray("data.updateUser.blogs").value.length should equal(0)
  }

  // UPSERTS //

  "An upsert with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id          String @id @default(cuid())
                                                |  first_name  String
                                                |  last_name   String
                                                |  
                                                |  @@unique([first_name, last_name])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val upsertQuery = s"""mutation {
                         |  upsertUser(where: {
                         |    first_name_last_name: {
                         |      first_name: "The"
                         |      last_name: "Dude"
                         |    }}
                         |    create: {
                         |      first_name: "The"
                         |      last_name: "Dude"
                         |    }
                         |    update: {
                         |      last_name: "Knight of Ni"
                         |    }) {
                         |    id
                         |    last_name
                         |  }
                         |}""".stripMargin

    val create_result = server.query(
      upsertQuery,
      project
    )

    create_result.pathAsString("data.upsertUser.last_name") should equal("Dude")

    val update_result = server.query(
      upsertQuery,
      project
    )

    update_result.pathAsString("data.upsertUser.last_name") should equal("Knight of Ni")
  }

  "A nested upsert with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id    String @id @default(cuid())
                                                |  name  String
                                                |  blogs Blog[]
                                                |}
                                                |
                                                |model Blog {
                                                |  id        String @id @default(cuid())
                                                |  title     String
                                                |  category  String
                                                |  author    User?
                                                |
                                                |  @@unique([title, category])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    name: "The Average Leddit User"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val upsertQuery = s"""mutation {
                         |  updateUser(where: {
                         |    id: "$userId"
                         |  }
                         |  data: {
                         |    blogs: {
                         |      upsert: {
                         |        where: {
                         |          title_category: {
                         |            title: "How to farm karma with puppy pictures"
                         |            category: "Pop Culture"
                         |          }
                         |        }
                         |        create: {
                         |          title: "How to farm karma with puppy pictures"
                         |          category: "Pop Culture"
                         |        },
                         |        update: {
                         |          category: "Drama"
                         |        }
                         |      }
                         |  }}) {
                         |    blogs {
                         |      id
                         |      category
                         |    }
                         |  }
                         |}""".stripMargin

    val create_result = server.query(
      upsertQuery,
      project
    )

    create_result.pathAsJsArray("data.updateUser.blogs").value.length should equal(1)

    val update_result = server.query(
      upsertQuery,
      project
    )

    update_result.pathAsJsArray("data.updateUser.blogs").value.head.pathAsString("category") should equal("Drama")
  }

  // SETS //

  "A nested set with a multi-field unique" should "work" in {
    val project = SchemaDsl.fromStringV11() { """model User {
                                                |  id    String @id @default(cuid())
                                                |  name  String
                                                |  blogs Blog[]
                                                |}
                                                |
                                                |model Blog {
                                                |  id        String @id @default(cuid())
                                                |  title     String
                                                |  category  String
                                                |  author    User?
                                                |
                                                |  @@unique([title, category])
                                                |}
                                              """.stripMargin }
    database.setup(project)

    val userId = server
      .query(
        s"""mutation {
           |  createUser(data: {
           |    name: "Ellen Ripley"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createUser.id")

    val blog1 = server
      .query(
        s"""mutation {
           |  createBlog(data: {
           |    title: "Aliens bad mmmkay"
           |    category: "Education"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createBlog.id")

    val blog2 = server
      .query(
        s"""mutation {
           |  createBlog(data: {
           |    title: "Cooking with Aliens"
           |    category: "Cooking"
           |  }) {
           |    id
           |  }
           |}""".stripMargin,
        project
      )
      .pathAsString("data.createBlog.id")

    val result = server
      .query(
        s"""mutation {
           |  updateUser(
           |    where: {
           |      id: "$userId"
           |    }
           |    data: {
           |      blogs:  {
           |        set: [{
           |          title_category: {
           |            title: "Cooking with Aliens"
           |            category: "Cooking"
           |          }
           |        }]
           |      }
           |  }) {
           |    id
           |    blogs {
           |      id
           |    }
           |  }
           |}""".stripMargin,
        project
      )

    result.pathAsJsArray("data.updateUser.blogs").value.head.pathAsString("id") should equal(blog2)
  }
}
