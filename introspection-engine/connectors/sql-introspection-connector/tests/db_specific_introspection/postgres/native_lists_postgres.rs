use crate::*;
use barrel::types;
use test_harness::*;

#[test_each_connector(tags("postgres"))]
async fn introspecting_native_arrays_should_work(api: &TestApi) {
    let barrel = api.barrel();
    let _setup_schema = barrel
        .execute(|migration| {
            migration.create_table("Post", |t| {
                t.add_column("id", types::primary());
                t.inject_custom("ints INTEGER [12]");
                t.inject_custom("bools BOOLEAN [12]");
                t.inject_custom("strings TEXT [12]");
                t.inject_custom("floats FLOAT [12]");
            });
        })
        .await;

    let dm = r#"
            datasource pg {
              provider = "postgres"
              url = "postgresql://localhost:5432"
            }

            model Post {
               id      Int @id @default(autoincrement())
               ints     Int []
               bools    Boolean []
               strings  String []
               floats   Float []
               }
        "#;
    let result = dbg!(api.introspect().await);
    custom_assert(&result, dm);
}
