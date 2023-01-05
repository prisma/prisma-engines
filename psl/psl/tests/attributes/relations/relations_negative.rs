use crate::{common::*, with_header, Provider};

#[test]
fn fail_if_ambiguous_relation_fields_do_not_specify_a_name() {
    let dml = indoc! {r#"
        model Todo {
          id Int @id
          comments Comment[]
          comments2 Comment[]
        }

        model Comment {
          id Int @id
          text String
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "Todo": Ambiguous relation detected. The fields `comments` and `comments2` in model `Todo` both refer to `Comment`. Please provide different relation names for them by adding `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mcomments Comment[][0m
        [1;94m 4 | [0m  comments2 Comment[]
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn fail_if_naming_relation_fields_the_same_as_the_explicit_names() {
    let dml = indoc! {r#"
        model Club {
          id                 Int      @id @default(autoincrement())
          adminId            Int      @map("admin_id")
          admin              User     @relation(fields: [adminId], references: [id])
          members            User[]   @relation("ClubToUser")

          @@map("clubs")
        }

        model User {
          id                 Int       @id @default(autoincrement())
          clubs_clubsTousers Club[]    @relation("ClubToUser")
          ownedClubs         Club[]

          @@map("users")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "Club": Ambiguous relation detected. The fields `admin` and `members` in model `Club` both refer to `User`. Please provide different relation names for them by adding `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  adminId            Int      @map("admin_id")
        [1;94m 4 | [0m  [1;91madmin              User     @relation(fields: [adminId], references: [id])[0m
        [1;94m 5 | [0m  members            User[]   @relation("ClubToUser")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn must_error_when_non_existing_fields_are_used() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          firstName String
          lastName String
          posts Post[]

          @@unique([firstName, lastName])
        }

        model Post {
          id   Int    @id
          text String
          user User   @relation(fields: [authorFirstName, authorLastName], references: [firstName, lastName])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating: The argument fields must refer only to existing fields. The following fields do not exist in this model: authorFirstName, authorLastName[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m  text String
        [1;94m13 | [0m  user User   @relation(fields: [1;91m[authorFirstName, authorLastName][0m, references: [firstName, lastName])
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_relations_with_automatic_names_1() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          posts Post[]
          more_posts Post[]
        }

        model Post {
          post_id Int @id
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": Ambiguous relation detected. The fields `posts` and `more_posts` in model `User` both refer to `Post`. Please provide different relation names for them by adding `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mposts Post[][0m
        [1;94m 4 | [0m  more_posts Post[]
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_colliding_implicit_self_relations() {
    let dml = indoc! {r#"
        model User {
          id          Int      @id @default(autoincrement())
          name        String?

          husband     User?    @relation("MarriagePartners")
          wife        User     @relation("MarriagePartners")

          teacher     User?    @relation("TeacherStudents")
          students    User[]   @relation("TeacherStudents")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `wife` on Model `User` and `husband` on Model `User` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m  husband     User?    @relation("MarriagePartners")
        [1;94m 6 | [0m  [1;91mwife        User     @relation("MarriagePartners")[0m
        [1;94m 7 | [0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `husband` on Model `User` and `wife` on Model `User` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m  [1;91mhusband     User?    @relation("MarriagePartners")[0m
        [1;94m 6 | [0m  wife        User     @relation("MarriagePartners")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `wife` on Model `User` and `husband` on Model `User` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m  husband     User?    @relation("MarriagePartners")
        [1;94m 6 | [0m  [1;91mwife        User     @relation("MarriagePartners")[0m
        [1;94m 7 | [0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `husband` on Model `User` and `wife` on Model `User` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m  [1;91mhusband     User?    @relation("MarriagePartners")[0m
        [1;94m 6 | [0m  wife        User     @relation("MarriagePartners")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `teacher` on Model `User` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m
        [1;94m 8 | [0m  [1;91mteacher     User?    @relation("TeacherStudents")[0m
        [1;94m 9 | [0m  students    User[]   @relation("TeacherStudents")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `teacher` on Model `User` must specify the `references` argument in the @relation attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m
        [1;94m 8 | [0m  [1;91mteacher     User?    @relation("TeacherStudents")[0m
        [1;94m 9 | [0m  students    User[]   @relation("TeacherStudents")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_relations_with_automatic_names_2() {
    // test case based on: https://github.com/prisma/prisma2/issues/976
    let dml = indoc! {r#"
        model User {
          id Int @id
          posts Post[]
        }

        model Post {
          post_id Int @id
          author1 User
          author2 User
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "Post": Ambiguous relation detected. The fields `author1` and `author2` in model `Post` both refer to `User`. Please provide different relation names for them by adding `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  post_id Int @id
        [1;94m 8 | [0m  [1;91mauthor1 User[0m
        [1;94m 9 | [0m  author2 User
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_relations_with_manual_names_1() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          posts Post[] @relation(name: "test")
          more_posts Post[] @relation(name: "test")
        }

        model Post {
          post_id Int @id
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": Wrongly named relation detected. The fields `posts` and `more_posts` in model `User` both use the same relation name. Please provide different relation names for them through `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mposts Post[] @relation(name: "test")[0m
        [1;94m 4 | [0m  more_posts Post[] @relation(name: "test")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_relations_with_manual_names_2() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          posts Post[] @relation(name: "a")
          more_posts Post[] @relation(name: "b")
          some_posts Post[]
          even_more_posts Post[] @relation(name: "a")
        }

        model Post {
          post_id Int @id
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": Wrongly named relation detected. The fields `posts` and `even_more_posts` in model `User` both use the same relation name. Please provide different relation names for them through `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mposts Post[] @relation(name: "a")[0m
        [1;94m 4 | [0m  more_posts Post[] @relation(name: "b")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_self_relation() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          father User
          son User
          mother User
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": Unnamed self relation detected. The fields `father`, `son` and `mother` in model `User` have no relation name. Please provide a relation name for one of them by adding `@relation(<name>).[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mfather User[0m
        [1;94m 4 | [0m  son User
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_self_relation_with_two_fields() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          child User
          mother User
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": Ambiguous self relation detected. The fields `child` and `mother` in model `User` both refer to `User`. If they are part of the same relation add the same relation name for them with `@relation(<name>)`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mchild User[0m
        [1;94m 4 | [0m  mother User
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_ambiguous_named_self_relation() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          father User @relation(name: "family")
          son User @relation(name: "family")
          mother User @relation(name: "family")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "User": Wrongly named self relation detected. The fields `father`, `son` and `mother` in model `User` have the same relation name. At most two relation fields can belong to the same relation and therefore have the same name. Please assign a different relation name to one of them.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mfather User @relation(name: "family")[0m
        [1;94m 4 | [0m  son User @relation(name: "family")
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_conflicting_back_relation_field_name() {
    let dml = indoc! {r#"
        model User {
          id Int @id
          posts Post[] @relation(name: "test")
          more_posts Post[]
        }

        model Post {
          post_id Int @id
          User User @relation(name: "test")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `more_posts` in model `User`: The relation field `more_posts` on model `User` is missing an opposite relation field on the model `Post`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  posts Post[] @relation(name: "test")
        [1;94m 4 | [0m  [1;91mmore_posts Post[][0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `User` on Model `Post` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  post_id Int @id
        [1;94m 9 | [0m  [1;91mUser User @relation(name: "test")[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `User` on Model `Post` must specify the `references` argument in the @relation attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  post_id Int @id
        [1;94m 9 | [0m  [1;91mUser User @relation(name: "test")[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]

//todo formatter should make an offer
fn should_fail_when_relation_attribute_is_missing_for_one_to_one_relations() {
    // Post is lower that User. So the references should be stored in Post.
    let dml = indoc! {r#"
        model User {
          user_id Int  @id
          post    Post
        }

        model Post {
          post_id Int  @id
          user    User
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `user` on Model `Post` and `post` on Model `User` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  post_id Int  @id
        [1;94m 8 | [0m  [1;91muser    User[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `post` on Model `User` and `user` on Model `Post` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  user_id Int  @id
        [1;94m 3 | [0m  [1;91mpost    Post[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `user` on Model `Post` and `post` on Model `User` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  post_id Int  @id
        [1;94m 8 | [0m  [1;91muser    User[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `post` on Model `User` and `user` on Model `Post` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  user_id Int  @id
        [1;94m 3 | [0m  [1;91mpost    Post[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_conflicting_generated_back_relation_fields() {
    // More specifically, this should not panic.
    let dml = indoc! {r#"
        model Todo {
          id Int @id
          author Owner @relation(name: "AuthorTodo")
          delegatedTo Owner? @relation(name: "DelegatedToTodo")
        }

        model Owner {
          id Int @id
          todos Todo[]
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `author` in model `Todo`: The relation field `author` on model `Todo` is missing an opposite relation field on the model `Owner`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mauthor Owner @relation(name: "AuthorTodo")[0m
        [1;94m 4 | [0m  delegatedTo Owner? @relation(name: "DelegatedToTodo")
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `delegatedTo` in model `Todo`: The relation field `delegatedTo` on model `Todo` is missing an opposite relation field on the model `Owner`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  author Owner @relation(name: "AuthorTodo")
        [1;94m 4 | [0m  [1;91mdelegatedTo Owner? @relation(name: "DelegatedToTodo")[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `todos` in model `Owner`: The relation field `todos` on model `Owner` is missing an opposite relation field on the model `Todo`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  id Int @id
        [1;94m 9 | [0m  [1;91mtodos Todo[][0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

//reformat implicit relations test files

//todo this talked about adding backrelation fields but was adding forward field + scalarfield
#[test]
fn must_generate_forward_relation_fields_for_named_relation_fields() {
    //reject, hint to prisma format, add scalar field and relation field, validate again
    let dml = indoc! {r#"
        model Todo {
          id Int @id
          assignees User[] @relation(name: "AssignedTodos")
        }

        model User {
          id Int @id
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `assignees` in model `Todo`: The relation field `assignees` on model `Todo` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91massignees User[] @relation(name: "AssignedTodos")[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

// todo this is also accepted and adds a postId scalar field under the hood on PostableEntity
// is almost the exact same case as the one above (minus the relationname), but reported as a bug and also understood by harshit as such
#[test]
fn issue4850() {
    //reject, hint to prisma format, add scalar field and relation field, validate again
    let dml = indoc! {r#"
        model PostableEntity {
          id String @id
        }

        model Post {
          id String @id
          postableEntities PostableEntity[]
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `postableEntities` in model `Post`: The relation field `postableEntities` on model `Post` is missing an opposite relation field on the model `PostableEntity`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:7[0m
        [1;94m   | [0m
        [1;94m 6 | [0m  id String @id
        [1;94m 7 | [0m  [1;91mpostableEntities PostableEntity[][0m
        [1;94m 8 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

//todo I think this should be fine and just add the @relation and relationname to the backrelation field
// but this interprets the dm as containing two relations.
#[test]
fn issue4822() {
    //reject, ask to name custom_Post relation
    let dml = indoc! {r#"
        model Post {
          id          Int    @id
          user_id     Int    @unique
          custom_User User   @relation("CustomName", fields: [user_id], references: [id])
        }

        model User {
          id          Int    @id
          custom_Post Post?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `custom_User` in model `Post`: The relation field `custom_User` on model `Post` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  user_id     Int    @unique
        [1;94m 4 | [0m  [1;91mcustom_User User   @relation("CustomName", fields: [user_id], references: [id])[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating field `custom_Post` in model `User`: The relation field `custom_Post` on model `User` is missing an opposite relation field on the model `Post`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m  id          Int    @id
        [1;94m 9 | [0m  [1;91mcustom_Post Post?[0m
        [1;94m10 | [0m}
        [1;94m   | [0m
    "#]];

    expect_error(dml, &expect);
}

#[test]
fn issue5216() {
    let dml = indoc! {r#"
        model user {
          id             String        @id
          email          String        @unique
          organization   organization? @relation(references: [id])
        }

        model organization {
          id        String   @id
          users     user[]
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `organization` on Model `user` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  email          String        @unique
        [1;94m 4 | [0m  [1;91morganization   organization? @relation(references: [id])[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

//todo this is also accepted but will under the hood point the createdBy relationfield to the same userId scalar
// as the user relationfield
// duplicate of 5540
// comment by matt:
// We don't want to remove the formatting feature that adds @relation and foreign key, this is a beloved feature.
// We want the validator to ensure that @relation always exists and links to a valid field.
// If the formatter is unable to correctly add @relation because of an ambiguity (e.g. user & createdBy), it shouldn't try. The validator will just tell you that you're missing @relation and need to add them in by hand to resolve the issue.
#[test]
fn issue5069() {
    // reject
    let dml = indoc! {r#"
        model Code {
          id          String        @id
          createdById String?
          createdBy   User?

          userId      String?
          user        User?         @relation("code", fields: [userId], references: [id])
        }

        model User {
          id         String         @id
          codes      Code[]         @relation("code")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `createdBy` in model `Code`: The relation field `createdBy` on model `Code` is missing an opposite relation field on the model `User`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  createdById String?
        [1;94m 4 | [0m  [1;91mcreatedBy   User?[0m
        [1;94m 5 | [0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn must_add_referenced_fields_on_both_sides_for_one_to_many_relations() {
    let dml = indoc! {r#"
        model User {
          user_id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          user    User
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `user` on Model `Post` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  post_id Int    @id
        [1;94m 8 | [0m  [1;91muser    User[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `user` on Model `Post` must specify the `references` argument in the @relation attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:8[0m
        [1;94m   | [0m
        [1;94m 7 | [0m  post_id Int    @id
        [1;94m 8 | [0m  [1;91muser    User[0m
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));

    // prove that lexicographic order does not have an influence.
    let dml = indoc! {r#"
        model User {
          user_id Int    @id
          post    Post
        }

        model Post {
          post_id Int    @id
          users   User[]
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `post` on Model `User` must specify the `fields` argument in the @relation attribute. You can run `prisma format` to fix this automatically.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  user_id Int    @id
        [1;94m 3 | [0m  [1;91mpost    Post[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation field `post` on Model `User` must specify the `references` argument in the @relation attribute.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  user_id Int    @id
        [1;94m 3 | [0m  [1;91mpost    Post[0m
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_on_missing_embed_ids_on_self_relations() {
    let dml = indoc! {r#"
        model Human {
          id Int @id
          father Human? @relation("paternity")
          son Human? @relation("paternity")
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `father` on Model `Human` and `son` on Model `Human` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mfather Human? @relation("paternity")[0m
        [1;94m 4 | [0m  son Human? @relation("paternity")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `son` on Model `Human` and `father` on Model `Human` do not provide the `fields` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  father Human? @relation("paternity")
        [1;94m 4 | [0m  [1;91mson Human? @relation("paternity")[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `father` on Model `Human` and `son` on Model `Human` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  id Int @id
        [1;94m 3 | [0m  [1;91mfather Human? @relation("paternity")[0m
        [1;94m 4 | [0m  son Human? @relation("paternity")
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The relation fields `son` on Model `Human` and `father` on Model `Human` do not provide the `references` argument in the @relation attribute. You have to provide it on one of the two fields.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  father Human? @relation("paternity")
        [1;94m 4 | [0m  [1;91mson Human? @relation("paternity")[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn mapping_foreign_keys_with_a_name_that_is_too_long_should_error() {
    let dml = with_header(
        indoc! {r#"
        model User {
          id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          user_id Int
          user    User   @relation(fields:[post_id], references: [id], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")
        }
    "#},
        Provider::Postgres,
        &[],
    );

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating model "Post": The constraint name 'IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits' specified in the `map` argument for the `@relation` constraint is too long for your chosen provider. The maximum allowed length is 63 bytes.[0m
          [1;94m-->[0m  [4mschema.prisma:19[0m
        [1;94m   | [0m
        [1;94m18 | [0m  user_id Int
        [1;94m19 | [0m  user    User   [1;91m@relation(fields:[post_id], references: [id], map: "IfYouAreGoingToPickTheNameYourselfYouShouldReallyPickSomethingShortAndSweetInsteadOfASuperLongNameViolatingLengthLimits")[0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(&dml));
}

#[test]
fn mapping_foreign_keys_on_sqlite_should_error() {
    let dml = indoc! {r#"
        datasource test {
          provider = "sqlite"
          url = "file:."
        }

        model User {
          id Int    @id
          posts   Post[]
        }

        model Post {
          post_id Int    @id
          user_id Int
          user    User   @relation(fields:[post_id], references: [id], map: "NoNamedForeignKeysOnSQLite")
        }
     "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Your provider does not support named foreign keys.[0m
          [1;94m-->[0m  [4mschema.prisma:14[0m
        [1;94m   | [0m
        [1;94m13 | [0m  user_id Int
        [1;94m14 | [0m  user    User   [1;91m@relation(fields:[post_id], references: [id], map: "NoNamedForeignKeysOnSQLite")[0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn relation_field_in_composite_type_errors() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            street String
            test Test
        }

        model Test {
            id Int @id
        }
    "#;

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating composite type "Address": Test refers to a model, making this a relation field. Relation fields inside composite types are not supported.[0m
          [1;94m-->[0m  [4mschema.prisma:9[0m
        [1;94m   | [0m
        [1;94m 8 | [0m            street String
        [1;94m 9 | [0m            test [1;91mTest[0m
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(schema));
}

#[test]
fn relation_attribute_on_a_composite_field_errors() {
    let schema = r#"
        datasource db {
            provider = "mongodb"
            url = "mongodb://"
        }

        type Address {
            street String
        }

        model Test {
            id Int @id
            addres Address? @relation("TestAddress")
        }
    "#;

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Invalid field type, not a relation.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m            id Int @id
        [1;94m13 | [0m            addres Address? [1;91m@relation("TestAddress")[0m
        [1;94m   | [0m
        [1;91merror[0m: [1mNo such argument.[0m
          [1;94m-->[0m  [4mschema.prisma:13[0m
        [1;94m   | [0m
        [1;94m12 | [0m            id Int @id
        [1;94m13 | [0m            addres Address? @relation([1;91m"TestAddress"[0m)
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(schema));
}

#[test]
fn a_typoed_relation_should_fail_gracefully() {
    let dml = indoc! {r#"
        datasource db {
          provider = "sqlserver"
          url      = env("DATABASE_URL")
        }

        model Test {
          id         Int        @id
          fk         Int
          testparent TestParent @relation(fields: [fk], references: [id])
        }

        model TestParent {
          id    Int    @id
          tests Test[]

          fk   Int
          self TestParent @relation(fields: [fk], references: [id])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError validating field `self` in model `TestParent`: The relation field `self` on model `TestParent` is missing an opposite relation field on the model `TestParent`. Either run `prisma format` or add it manually.[0m
          [1;94m-->[0m  [4mschema.prisma:17[0m
        [1;94m   | [0m
        [1;94m16 | [0m  fk   Int
        [1;94m17 | [0m  [1;91mself TestParent @relation(fields: [fk], references: [id])[0m
        [1;94m18 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn cannot_use_references_with_many_to_many_relations() {
    let dml = indoc! {r#"
        model User {
          id      Int    @id
          post_id Int
          posts   Post[] @relation("a", references: [id])
        }

        model Post {
          id      Int    @id
          user_id Int
          users   User[] @relation("a", references: [id])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": Implicit many-to-many relation should not have references argument defined. Either remove it, or change the relation to one-to-many.[0m
          [1;94m-->[0m  [4mschema.prisma:10[0m
        [1;94m   | [0m
        [1;94m 9 | [0m  user_id Int
        [1;94m10 | [0m  [1;91musers   User[] @relation("a", references: [id])[0m
        [1;94m11 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": Implicit many-to-many relation should not have references argument defined. Either remove it, or change the relation to one-to-many.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  post_id Int
        [1;94m 4 | [0m  [1;91mposts   Post[] @relation("a", references: [id])[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_if_not_using_unique_constraint_with_singular_one_to_one() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          fk Int?
          b  B?   @relation(fields: [fk], references: [id])
        }

        model B {
          id Int @id
          a  A?
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A one-to-one relation must use unique fields on the defining side. Either add an `@unique` attribute to the field `fk`, or change the relation to one-to-many.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  fk Int?
        [1;94m 4 | [0m  [1;91mb  B?   @relation(fields: [fk], references: [id])[0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_if_not_using_unique_constraint_with_compound_one_to_one() {
    let dml = indoc! {r#"
        model A {
          id Int  @id
          fk1 Int?
          fk2 Int?
          b   B?   @relation(fields: [fk1, fk2], references: [id1, id2])
        }

        model B {
          id1 Int
          id2 Int
          a   A?

          @@id([id1, id2])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": A one-to-one relation must use unique fields on the defining side. Either add an `@@unique([fk1, fk2])` attribute to the model, or change the relation to one-to-many.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m  fk2 Int?
        [1;94m 5 | [0m  [1;91mb   B?   @relation(fields: [fk1, fk2], references: [id1, id2])[0m
        [1;94m 6 | [0m}
        [1;94m   | [0m
    "#]];

    expect.assert_eq(&parse_unwrap_err(dml));
}

#[test]
fn should_fail_if_not_using_unique_constraint_with_single_one_to_many() {
    let dml = indoc! {r#"
        model A {
          id         Int          @id @default(autoincrement())
          custom_id  String
          B          B[]

          @@index([custom_id])
        }

        model B {
          id        Int     @id @default(autoincrement())
          a_id String
          A         A @relation(fields: [a_id], references: [custom_id])
        }
    "#};

    let expect = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The argument `references` must refer to a unique criterion in the related model. Consider adding an `@unique` attribute to the field `custom_id` in the model `A`.[0m
          [1;94m-->[0m  [4mschema.prisma:22[0m
        [1;94m   | [0m
        [1;94m21 | [0m  a_id String
        [1;94m22 | [0m  [1;91mA         A @relation(fields: [a_id], references: [custom_id])[0m
        [1;94m23 | [0m}
        [1;94m   | [0m
    "#]];

    let dml = with_header(dml, Provider::Mysql, &[]);

    expect.assert_eq(&parse_unwrap_err(&dml));
}

#[test]
fn multiple_relation_validation_errors_do_not_prevent_each_other_across_models() {
    let schema = r#"
        generator client {
          provider      = "prisma-client-js"
          binaryTargets = ["darwin"]
        }

        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        model Post {
          POST_ID            Int  @id @db.UnsignedInt
          USER_NON_UNIQUE_ID Int  @db.UnsignedInt
          User               User @relation(fields: [USER_NON_UNIQUE_ID], references: [USER_NON_UNIQUE_ID], onUpdate: Restrict, map: "FK_USER_NON_UNIQUE_ID")

          @@index([USER_NON_UNIQUE_ID], map: "FK_USER_NON_UNIQUE_ID")
        }

        model User {
          USER_ID            Int    @id @default(autoincrement()) @db.UnsignedInt
          USER_NON_UNIQUE_ID Int    @db.UnsignedInt
          Post               Post[]

          @@index([USER_NON_UNIQUE_ID], map: "index_name")
        }

        model stock {
          STOCK_ID            Int           @id @default(autoincrement()) @db.UnsignedInt
          STOCK_NON_UNIQUE_ID Int           @db.UnsignedInt
          stock_detail        stock_detail?

          @@index([STOCK_NON_UNIQUE_ID], map: "index_name")
        }

        model stock_detail {
          STOCK_ID Int   @id @db.UnsignedInt
          stock    stock @relation(fields: [STOCK_ID], references: [STOCK_NON_UNIQUE_ID], onUpdate: Restrict, map: "FK_STOCK_NON_UNIQUE_ID")
        }
    "#;

    let expected_error = expect![[r#"
        [1;91merror[0m: [1mError parsing attribute "@relation": The argument `references` must refer to a unique criterion in the related model. Consider adding an `@unique` attribute to the field `USER_NON_UNIQUE_ID` in the model `User`.[0m
          [1;94m-->[0m  [4mschema.prisma:15[0m
        [1;94m   | [0m
        [1;94m14 | [0m          USER_NON_UNIQUE_ID Int  @db.UnsignedInt
        [1;94m15 | [0m          [1;91mUser               User @relation(fields: [USER_NON_UNIQUE_ID], references: [USER_NON_UNIQUE_ID], onUpdate: Restrict, map: "FK_USER_NON_UNIQUE_ID")[0m
        [1;94m16 | [0m
        [1;94m   | [0m
        [1;91merror[0m: [1mError parsing attribute "@relation": The argument `references` must refer to a unique criterion in the related model. Consider adding an `@unique` attribute to the field `STOCK_NON_UNIQUE_ID` in the model `stock`.[0m
          [1;94m-->[0m  [4mschema.prisma:38[0m
        [1;94m   | [0m
        [1;94m37 | [0m          STOCK_ID Int   @id @db.UnsignedInt
        [1;94m38 | [0m          [1;91mstock    stock @relation(fields: [STOCK_ID], references: [STOCK_NON_UNIQUE_ID], onUpdate: Restrict, map: "FK_STOCK_NON_UNIQUE_ID")[0m
        [1;94m39 | [0m        }
        [1;94m   | [0m
    "#]];

    expect_error(schema, &expected_error)
}
