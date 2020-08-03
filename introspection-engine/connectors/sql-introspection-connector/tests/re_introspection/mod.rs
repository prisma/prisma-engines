use crate::*;
use crate::{custom_assert, test_each_connector, TestApi};
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_manually_overwritten_mapped_model_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("_User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model Custom_User {
               id               Int         @id @default(autoincrement())
               
               @@map(name: "_User")
            }
        "#;

    let final_dm = r#"  
            model Custom_User {
               id               Int         @id @default(autoincrement()) 
               
               @@map(name: "_User")
            }
              
            model Unrelated {
               id               Int         @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":7,\"message\":\"These models were enriched with `@@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"Custom_User\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_manually_overwritten_mapped_field_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.add_column("_test", types::integer());
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               id               Int         @id @default(autoincrement())
               custom_test      Int         @map("_test")
            }
        "#;

    let final_dm = r#"
            model User {
               id               Int         @id @default(autoincrement()) 
               custom_test      Int         @map("_test")
            }  
            
            model Unrelated {
               id               Int         @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":8,\"message\":\"These fields were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"User\",\"field\":\"custom_test\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_mapped_model_and_field_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("User", "id").nullable(false));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model Post {
               id               Int         @id @default(autoincrement())
               c_user_id        Int         @map("user_id")
               Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            }
            
            model Custom_User {
               c_id             Int         @id @default(autoincrement()) @map("id")
               Post             Post[]
               
               @@map(name: "User")
            }
        "#;

    let final_dm = r#"
            model Post {
               id               Int         @id @default(autoincrement())
               c_user_id        Int         @map("user_id")
               Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            }

            model Custom_User {
               c_id             Int         @id @default(autoincrement()) @map("id")
               Post             Post[]
               
               @@map(name: "User")
            }  
            
            model Unrelated {
               id               Int         @id @default(autoincrement())
            }          
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":7,\"message\":\"These models were enriched with `@@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"Custom_User\"}]},{\"code\":8,\"message\":\"These fields were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"Post\",\"field\":\"c_user_id\"},{\"model\":\"Custom_User\",\"field\":\"c_id\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_manually_mapped_model_and_field_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("_User", |t| {
                t.add_column("_id", types::primary());
            });

            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("_User", "_id").nullable(false));
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model Post {
               id               Int         @id @default(autoincrement())
               c_user_id        Int         @map("user_id")
               Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            }
            
            model Custom_User {
               c_id             Int         @id @default(autoincrement()) @map("_id")
               Post             Post[]
               
               @@map(name: "_User")
            }
        "#;

    let final_dm = r#"
            model Post {
               id               Int         @id @default(autoincrement())
               c_user_id        Int         @map("user_id")
               Custom_User      Custom_User @relation(fields: [c_user_id], references: [c_id])
            }
            
            model Custom_User {
               c_id             Int         @id @default(autoincrement()) @map("_id")
               Post             Post[]
               
               @@map(name: "_User")
            }  
                        
            model Unrelated {
               id               Int         @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":7,\"message\":\"These models were enriched with `@@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"Custom_User\"}]},{\"code\":8,\"message\":\"These fields were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"Post\",\"field\":\"c_user_id\"},{\"model\":\"Custom_User\",\"field\":\"c_id\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_mapped_field_name(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id_1", types::integer());
                t.add_column("id_2", types::integer());
                t.add_column("index", types::integer());
                t.add_column("unique_1", types::integer());
                t.add_column("unique_2", types::integer());
                t.inject_custom("Unique( \"unique_1\", \"unique_2\")");
                t.inject_custom("CONSTRAINT \"id\" PRIMARY KEY( \"id_1\", \"id_2\")");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    api.database()
        .execute_raw(
            &format!("CREATE INDEX test2 ON \"{}\".\"User\" (\"index\");", api.schema_name()),
            &[],
        )
        .await
        .unwrap();

    let input_dm = r#"
            model User { 
                c_id_1      Int     @map("id_1")
                id_2        Int
                c_index     Int     @map("index")
                c_unique_1  Int     @map("unique_1") 
                unique_2    Int
                    
                @@id([c_id_1, id_2])
                @@index([c_index], name: "test2")
                @@unique([c_unique_1, unique_2], name: "User_unique_1_unique_2_key")
            }
        "#;

    let final_dm = r#"
            model User { 
                c_id_1      Int     @map("id_1")
                id_2        Int
                c_index     Int     @map("index")
                c_unique_1  Int     @map("unique_1") 
                unique_2    Int
                    
                @@id([c_id_1, id_2])
                @@index([c_index], name: "test2")
                @@unique([c_unique_1, unique_2], name: "User_unique_1_unique_2_key")
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);

    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":8,\"message\":\"These fields were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"model\":\"User\",\"field\":\"c_id_1\"},{\"model\":\"User\",\"field\":\"c_index\"},{\"model\":\"User\",\"field\":\"c_unique_1\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_mapped_enum_name(api: &TestApi) {
    let sql = format!("CREATE Type color as ENUM ( 'black', 'white')");
    api.database().execute_raw(&sql, &[]).await.unwrap();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  color Not Null");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite            
            }
            
            enum BlackNWhite{
                black
                white
                
                @@map("color")
            }
        "#;

    let final_dm = r#"
             model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite            
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            enum BlackNWhite{
                black
                white
                
                @@map("color")
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":9,\"message\":\"These enums were enriched with `@@map` information taken from the previous Prisma schema.\",\"affected\":[{\"enm\":\"BlackNWhite\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_mapped_enum_value_name(api: &TestApi) {
    let sql = format!("CREATE Type color as ENUM ( 'black', 'white')");
    api.database().execute_raw(&sql, &[]).await.unwrap();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  color Not Null Default('black')");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               color            color @default(BLACK)            
            }
            
            enum color{
                BLACK @map("black")
                white
            }
        "#;

    let final_dm = r#"
             model User {
               id               Int @id @default(autoincrement())
               color            color @default(BLACK)            
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }

            enum color{
                BLACK @map("black")
                white
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":10,\"message\":\"These enum values were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"enm\":\"color\",\"value\":\"BLACK\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_manually_remapped_enum_value_name(api: &TestApi) {
    let sql = format!("CREATE Type color as ENUM ( '_black', 'white')");
    api.database().execute_raw(&sql, &[]).await.unwrap();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  color Not Null Default('_black')");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               color            color @default(BLACK)            
            }
            
            enum color{
                BLACK @map("_black")
                white
            }
        "#;

    let final_dm = r#"
             model User {
               id               Int @id @default(autoincrement())
               color            color @default(BLACK)            
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            enum color{
                BLACK @map("_black")
                white
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":10,\"message\":\"These enum values were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"enm\":\"color\",\"value\":\"BLACK\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_manually_re_mapped_enum_name(api: &TestApi) {
    let sql = format!("CREATE Type _color as ENUM ( 'black', 'white')");
    api.database().execute_raw(&sql, &[]).await.unwrap();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("color  _color Not Null");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite            
            }
            
            enum BlackNWhite{
                black
                white
                
                @@map("_color")
            }
        "#;

    let final_dm = r#" 
             model User {
               id               Int @id @default(autoincrement())
               color            BlackNWhite            
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            enum BlackNWhite{
                black
                white
                
                @@map("_color")
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":9,\"message\":\"These enums were enriched with `@@map` information taken from the previous Prisma schema.\",\"affected\":[{\"enm\":\"BlackNWhite\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_manually_re_mapped_invalid_enum_values(api: &TestApi) {
    let sql = format!("CREATE Type invalid as ENUM ( '@', '-')");
    api.database().execute_raw(&sql, &[]).await.unwrap();
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("sign  invalid Not Null");
            });

            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model User {
               id               Int @id @default(autoincrement())
               sign             invalid           
            }
            
            enum invalid{
                dash    @map("-")
                at      @map("@")
            }
        "#;

    let final_dm = r#" 
              model User {
               id               Int @id @default(autoincrement())
               sign             invalid           
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
            
            enum invalid{
                dash    @map("-")
                at      @map("@")
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
    let warnings = api.re_introspect_warnings(input_dm).await;

    assert_eq_json(&warnings, "[{\"code\":10,\"message\":\"These enum values were enriched with `@map` information taken from the previous Prisma schema.\",\"affected\":[{\"enm\":\"invalid\",\"value\":\"dash\"},{\"enm\":\"invalid\",\"value\":\"at\"}]}]");
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_multiple_changed_relation_names(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Employee", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Schedule", |t| {
                t.add_column("id", types::primary());
                t.add_column("morningEmployeeId", types::foreign("Employee", "id"));
                t.add_column("eveningEmployeeId", types::foreign("Employee", "id"));
            });
            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model Employee {
                  id                                            Int         @default(autoincrement()) @id
                  A                                             Schedule[]  @relation("EmployeeToSchedule_eveningEmployeeId")
                  Schedule_EmployeeToSchedule_morningEmployeeId Schedule[]  @relation("EmployeeToSchedule_morningEmployeeId")
            }
            
            model Schedule {
                  id                                            Int         @default(autoincrement()) @id
                  morningEmployeeId                             Int
                  eveningEmployeeId                             Int
                  Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id])
                  Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id])
            }
        "#;

    let final_dm = r#"
             model Employee {
                  id                                            Int         @default(autoincrement()) @id
                  A                                             Schedule[]  @relation("EmployeeToSchedule_eveningEmployeeId")
                  Schedule_EmployeeToSchedule_morningEmployeeId Schedule[]  @relation("EmployeeToSchedule_morningEmployeeId")
            }
            
            model Schedule {
                  id                                            Int         @default(autoincrement()) @id
                  morningEmployeeId                             Int
                  eveningEmployeeId                             Int
                  Employee_EmployeeToSchedule_eveningEmployeeId Employee    @relation("EmployeeToSchedule_eveningEmployeeId", fields: [eveningEmployeeId], references: [id])
                  Employee_EmployeeToSchedule_morningEmployeeId Employee    @relation("EmployeeToSchedule_morningEmployeeId", fields: [morningEmployeeId], references: [id])
            }

            model Unrelated {
               id               Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_custom_virtual_relation_field_names(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("User", "id").nullable(false).unique(true));
            });
            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
             model Post {
               id               Int @id @default(autoincrement())
               user_id          Int  @unique
               custom_User      User @relation(fields: [user_id], references: [id])
            }

            model User {
               id               Int @id @default(autoincrement())
               custom_Post      Post?
            }
        "#;

    let final_dm = r#"
             model Post {
               id               Int @id @default(autoincrement())
               user_id          Int  @unique
               custom_User      User @relation(fields: [user_id], references: [id])
            }
 
            model User {
               id               Int @id @default(autoincrement())
               custom_Post      Post?
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
           
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_custom_model_order(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("A", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("B", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("J", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("F", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Z", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("M", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("L", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
            model B {
               id               Int @id @default(autoincrement())
            }
             
            model A {
               id               Int @id @default(autoincrement())
            }
            
            model F {
               id               Int @id @default(autoincrement())
            }
             
            model C {
               id               Int @id @default(autoincrement())
            }
            
            model J {
               id               Int @id @default(autoincrement())
            }
             
            model Z {
               id               Int @id @default(autoincrement())
            }
            
            model K {
               id               Int @id @default(autoincrement())
            }
        "#;

    let final_dm = r#"
            model B {
               id               Int @id @default(autoincrement())
            }
             
            model A {
               id               Int @id @default(autoincrement())
            }
            
            model F {
               id               Int @id @default(autoincrement())
            }
             
            model J {
               id               Int @id @default(autoincrement())
            }
            
            model Z {
               id               Int @id @default(autoincrement())
            }
             
            model L {
               id               Int @id @default(autoincrement())
            }
            
            model M {
               id               Int @id @default(autoincrement())
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_custom_enum_order(api: &TestApi) {
    let sql = format!("CREATE Type a as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let sql = format!("CREATE Type b as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let sql = format!("CREATE Type j as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let sql = format!("CREATE Type f as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let sql = format!("CREATE Type z as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let sql = format!("CREATE Type m as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let sql = format!("CREATE Type l as ENUM ( 'id')");
    api.database().execute_raw(&sql, &[]).await.unwrap();

    let input_dm = r#"
            enum b {
               id
            }
             
            enum a {
               id
            }
            
            enum f {
               id
            }
             
            enum c {
               id
            }
            
            enum j {
               id
            }
             
            enum z {
               id
            }
            
            enum k {
               id
            }
        "#;

    let final_dm = r#"
            enum b {
               id
            }
             
            enum a {
               id
            }
            
            enum f {
               id
            }
             
            enum j {
               id
            }
            
            enum z {
               id
            }
             
            enum l {
               id
            }
            
            enum m {
               id
            }
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

#[test_each_connector(tags("postgres"))]
async fn re_introspecting_multiple_changed_relation_names_due_to_mapped_models(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.add_column("user_id", types::foreign("User", "id").nullable(false).unique(true));
                t.add_column("user_id2", types::foreign("User", "id").nullable(false).unique(true));
            });
            migration.create_table("Unrelated", |t| {
                t.add_column("id", types::primary());
            });
        })
        .await;

    let input_dm = r#"
             model Post {
               id               Int @id @default(autoincrement())
               user_id          Int  @unique
               user_id2         Int  @unique
               custom_User      Custom_User @relation("OtherUserToPost_user_id", fields: [user_id], references: [id])
               custom_User2     Custom_User @relation("OtherUserToPost_user_id2", fields: [user_id2], references: [id])
            }

            model Custom_User {
               id               Int @id @default(autoincrement())
               custom_Post      Post? @relation("OtherUserToPost_user_id")
               custom_Post2     Post? @relation("OtherUserToPost_user_id2")
               
               @@map("User")
            }
        "#;

    let final_dm = r#"
             model Post {
               id               Int @id @default(autoincrement())
               user_id          Int  @unique
               user_id2         Int  @unique
               custom_User      Custom_User @relation("Custom_UserToPost_user_id", fields: [user_id], references: [id])
               custom_User2     Custom_User @relation("Custom_UserToPost_user_id2", fields: [user_id2], references: [id])
            }

            model Custom_User {
               id               Int @id @default(autoincrement())
               custom_Post      Post? @relation("Custom_UserToPost_user_id")
               custom_Post2     Post? @relation("Custom_UserToPost_user_id2")
               
               @@map("User")
            }
            
            model Unrelated {
               id               Int @id @default(autoincrement())
            }
           
        "#;
    let result = dbg!(api.re_introspect(input_dm).await);
    custom_assert(&result, final_dm);
}

// #[test_each_connector(tags("postgres"))]
// async fn re_introspecting_virtual_default(api: &TestApi) {
//     let barrel = api.barrel();
//     let _setup_schema = barrel
//         .execute(|migration| {
//             migration.create_table("User", |t| {
//                 t.add_column("id", types::primary());
//                 t.add_column("text", types::text());
//             });
//             migration.create_table("Unrelated", |t| {
//                 t.add_column("id", types::primary());
//             });
//         })
//         .await;
//
//     let input_dm = r#"
//             model User {
//                id        Int    @id @default(autoincrement())
//                text      String @default("virtual_default")
//             }
//         "#;
//
//     let final_dm = r#"
//             model Unrelated {
//                id               Int @id @default(autoincrement())
//             }
//
//              model User {
//                id        Int    @id @default(autoincrement())
//                text      String @default("virtual_default")
//             }
//         "#;
//     let result = dbg!(api.re_introspect(input_dm).await);
//     custom_assert(&result, final_dm);
// }
