use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_for_one_to_one_relations(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();

    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE([id], [age])");
            });
            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.inject_custom(&format!(
                    "FOREIGN KEY ([user_id],[user_age]) REFERENCES [{}].[User]([id], [age])",
                    schema
                ));
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE([user_id], [user_age])");
            });
        })
        .await;

    let dm = r#"          
            model Post {
                id       Int   @id @default(autoincrement())
                user_id  Int?
                user_age Int?
                User     User? @relation(fields: [user_id, user_age], references: [id, age])
                    
                @@unique([user_id, user_age], name: "post_user_unique")
            }

            model User {
                id   Int   @id @default(autoincrement())
                age  Int
                Post Post?
                            
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_for_required_one_to_one_relations(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE([id], [age])");
            });
            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom(&format!(
                    "FOREIGN KEY ([user_id],[user_age]) REFERENCES [{}].[User]([id], [age])",
                    schema,
                ));
                t.inject_custom("CONSTRAINT post_user_unique UNIQUE([user_id], [user_age])");
            });
        })
        .await;

    let dm = r#"
             model Post {
                id       Int  @id @default(autoincrement())
                user_id  Int
                user_age Int
                User     User @relation(fields: [user_id, user_age], references: [id, age])
                
                @@unique([user_id, user_age], name: "post_user_unique")
            }
            
            
            model User {
               id   Int   @id @default(autoincrement())
               age  Int
               Post Post?
               
               @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_for_one_to_many_relations(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE([id], [age])");
            });
            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer().nullable(true));
                t.add_column("user_age", types::integer().nullable(true));
                t.inject_custom(&format!(
                    "FOREIGN KEY ([user_id],[user_age]) REFERENCES [{}].[User]([id], [age])",
                    schema
                ));
            });
        })
        .await;

    let dm = r#"           
            model Post {
                id       Int   @id @default(autoincrement())
                user_id  Int?
                user_age Int?
                User     User? @relation(fields: [user_id, user_age], references: [id, age])
            }
                      
            model User {
                id   Int    @id @default(autoincrement())
                age  Int
                Post Post[]
                            
                @@unique([id, age], name: "user_unique")
            }
            
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_for_required_one_to_many_relations(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE([id], [age])");
            });
            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom(&format!(
                    "FOREIGN KEY ([user_id],[user_age]) REFERENCES [{}].[User]([id], [age])",
                    schema
                ));
            });
        })
        .await;

    let dm = r#"
            model Post {
                id       Int  @id @default(autoincrement())
                user_id  Int
                user_age Int
                User     User @relation(fields: [user_id, user_age], references: [id, age])
            }
            
            model User {
                id   Int    @id @default(autoincrement())
                age  Int
                Post Post[]
                
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_for_required_self_relations(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer());
                t.add_column("partner_age", types::integer());
                t.inject_custom(&format!(
                    "FOREIGN KEY ([partner_id],[partner_age]) REFERENCES [{}].[Person]([id], [age])",
                    schema
                ));
                t.inject_custom("CONSTRAINT [person_unique] UNIQUE ([id], [age])");
            });
        })
        .await;

    let dm = r#"
           model Person {
                id           Int      @id @default(autoincrement())
                age          Int
                partner_id   Int
                partner_age  Int
                Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
                other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")
                        
                @@unique([id, age], name: "person_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
#[test]
async fn compound_foreign_keys_should_work_for_self_relations(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().nullable(true));
                t.add_column("partner_age", types::integer().nullable(true));
                t.inject_custom(&format!(
                    "FOREIGN KEY ([partner_id],[partner_age]) REFERENCES [{}].[Person]([id], [age])",
                    schema
                ));
                t.inject_custom("CONSTRAINT [person_unique] UNIQUE ([id], [age])");
            });
        })
        .await;

    let dm = r#"
           model Person {
                id           Int      @id @default(autoincrement())
                age          Int
                partner_id   Int?
                partner_age  Int?
                Person       Person?  @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
                other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")
                
                @@unique([id, age], name: "person_unique")
            }   
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_with_defaults(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("Person", move |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.add_column("partner_id", types::integer().default(0));
                t.add_column("partner_age", types::integer().default(0));
                t.inject_custom(&format!(
                    "FOREIGN KEY ([partner_id],[partner_age]) REFERENCES [{}].[Person]([id], [age])",
                    schema
                ));
                t.inject_custom("CONSTRAINT [person_unique] UNIQUE ([id], [age])");
            });
        })
        .await;

    let dm = r#"
             model Person {
                id           Int      @id @default(autoincrement())
                age          Int
                partner_id   Int      @default(0)
                partner_age  Int      @default(0)
                Person       Person   @relation("PersonToPerson_partner_id_partner_age", fields: [partner_id, partner_age], references: [id, age])
                other_Person Person[] @relation("PersonToPerson_partner_id_partner_age")
                
                @@unique([id, age], name: "person_unique")
            }           
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn compound_foreign_keys_should_work_for_one_to_many_relations_with_non_unique_index(api: &TestApi) {
    let schema = api.schema_name().to_string();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(move |migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("age", types::integer());
                t.inject_custom("CONSTRAINT user_unique UNIQUE([id], [age])");
            });
            migration.create_table("Post", move |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::integer());
                t.add_column("user_age", types::integer());
                t.inject_custom(&format!(
                    "FOREIGN KEY ([user_id],[user_age]) REFERENCES [{}].[User]([id], [age])",
                    schema
                ));
            });
        })
        .await;

    let dm = r#"
            model Post {
                id       Int  @id @default(autoincrement())
                user_id  Int
                user_age Int
                User     User @relation(fields: [user_id, user_age], references: [id, age])
            }
                      
            model User {
                id   Int    @id @default(autoincrement())
                age  Int
                Post Post[]
                            
                @@unique([id, age], name: "user_unique")
            }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
