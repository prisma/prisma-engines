use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_one_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age`)");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`, `user_age`)");
            });
        })
        .await;

    let dm = r#"          
            model Post {
                id       Int   @default(autoincrement()) @id
                user_id  Int?
                user_age Int?
                User     User? @relation(fields: [user_id, user_age], references: [id, age])
                    
                @@unique([user_id, user_age], name: "post_user_unique")
            }

            model User {
                id   Int   @default(autoincrement()) @id
                age  Int
                Post Post?
                            
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_required_one_to_one_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age`)");
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE(`user_id`, `user_age`)");
            });
        })
        .await;

    let dm = r#"
             model Post {
                id       Int  @default(autoincrement()) @id
                user_id  Int
                user_age Int
                User     User @relation(fields: [user_id, user_age], references: [id, age])
                
                @@unique([user_id, user_age], name: "post_user_unique")
            }
            
            
            model User {
               id   Int   @default(autoincrement()) @id
               age  Int
               Post Post?
               
               @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_many_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age`)");
            });
        })
        .await;

    let dm = r#"           
            model Post {
                id       Int   @default(autoincrement()) @id
                user_id  Int?
                user_age Int?
                User     User? @relation(fields: [user_id, user_age], references: [id, age])
                    
                @@index([user_id, user_age], name: "user_id")
            }
                      
            model User {
                id   Int    @default(autoincrement()) @id
                age  Int
                Post Post[]
                            
                @@unique([id, age], name: "user_unique")
            }
            
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_required_one_to_many_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age`)");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id       Int  @default(autoincrement()) @id
                user_id  Int
                user_age Int
                User     User @relation(fields: [user_id, user_age], references: [id, age])
                
                @@index([user_id, user_age], name: "user_id")
            }
            
            model User {
                id   Int    @default(autoincrement()) @id
                age  Int
                Post Post[]
                
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_required_self_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Person", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());
                t.inject_custom("FOREIGN KEY (`partner_id`,`partner_age`) REFERENCES `Person`(`id`, `age`)");
                t.inject_custom("CONSTRAINT `person_unique` UNIQUE (`id`, `age`)");
            });
        })
        .await;

    let dm = r#"
           model Person {
                id           Int      @default(autoincrement()) @id
                age          Int
                partner_id   Int
                partner_age  Int
                Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
                other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")
                        
                @@index([partner_id, partner_age], name: "partner_id")
                @@unique([id, age], name: "person_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_self_relations(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Person", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().nullable(true));
                t.add_column("partner_age", types::integer().nullable(true));
                t.inject_custom("FOREIGN KEY (`partner_id`,`partner_age`) REFERENCES `Person`(`id`, `age`)");
                t.inject_custom("CONSTRAINT `person_unique` UNIQUE (`id`, `age`)");
            });
        })
        .await;

    let dm = r#"
           model Person {
                id           Int      @default(autoincrement()) @id
                age          Int
                partner_id   Int?
                partner_age  Int?
                Person       Person?  @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
                other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")
                
                @@index([partner_id, partner_age], name: "partner_id")
                @@unique([id, age], name: "person_unique")
            }   
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_with_defaults(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Person", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().default(0));
                t.add_column("partner_age", types::integer().default(0));
                t.inject_custom("FOREIGN KEY (`partner_id`,`partner_age`) REFERENCES `Person`(`id`, `age`)");
                t.inject_custom("CONSTRAINT `person_unique` UNIQUE (`id`, `age`)");
            });
        })
        .await;

    let dm = r#"
             model Person {
                id           Int      @default(autoincrement()) @id
                age          Int
                partner_id   Int      @default(0)
                partner_age  Int      @default(0)
                Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
                other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")
                
                @@index([partner_id, partner_age], name: "partner_id")
                @@unique([id, age], name: "person_unique")
            }           
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

//todo decide on this,
// this can at most be a one:one relation, but with a more limited subset of available connections
// fetch this from indexes
// what about separate uniques? all @unique == @@unique ?? No! separate ones do not fully work since you can only connect to a subset of the @@unique case
// model.indexes contains a multi-field unique index that matches the colums exactly, then it is unique
// if there are separate uniques it probably should not become a relation
// what breaks by having an @@unique that refers to fields that do not have a representation on the model anymore due to the merged relation field?
//#[test_each_connector(tags("mysql"))]
//#[test]
//async fn compound_foreign_keys_should_work_for_one_to_one_relations_with_separate_uniques(api: &TestApi) {
//    let barrel = api.barrel();
//    let _setup_schema = barrel
//        .execute(|migration| {
//            migration.create_table("User", |t| {
//                t.add_column("id", types::primary());
//                t.add_column("age", types::integer());
//                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
//            });
//            migration.create_table("Post", |t| {
//                t.add_column("id", types::primary());
//                t.add_column("user_id", types::integer().unique(true));
//                t.add_column("user_age", types::integer().unique(true));
//                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age`)");
//            });
//        })
//        .await;
//
//    let dm = r#"
//            model Post {
//                id      Int                 @id
//                user    User                @map(["user_id", "user_age"]) @relation(references:[id, age])
//            }
//
//            model User {
//               age      Int
//               id       Int                 @id
//               post     Post?
//
//               @@unique([id, age], name: "user_unique")
//            }
//        "#;
//    let result = dbg!(api.introspect().await);
//    custom_assert(&result, dm);
//}

#[test_each_connector(tags("mysql"))]
#[test]
async fn compound_foreign_keys_should_work_for_one_to_many_relations_with_non_unique_index(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE(`id`, `age`)");
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom("FOREIGN KEY (`user_id`,`user_age`) REFERENCES `User`(`id`, `age`)");
            });
        })
        .await;

    let dm = r#"
            model Post {
                id       Int  @default(autoincrement()) @id
                user_id  Int
                user_age Int
                User     User @relation(fields: [user_id, user_age], references: [id, age])
                    
                @@index([user_id, user_age], name: "user_id")
            }
                      
            model User {
                id   Int    @default(autoincrement()) @id
                age  Int
                Post Post[]
                            
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
