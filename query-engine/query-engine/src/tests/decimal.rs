use super::test_api::*;
use indoc::indoc;
use serde_json::json;
use test_macros::test_each_connector_mssql as test_each_connector;

static MODEL: &str = indoc! {"
    model Transaction {
        id       Int    @id @default(autoincrement())
        amount   Float
    }
"};

#[test_each_connector]
async fn decimal_conversion(api: &TestApi) -> anyhow::Result<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();
    let query_engine = api.create_engine(&MODEL).await?;

    let query = indoc! {r#"
        mutation {
            createOneTransaction(data: {amount: 1.59283191 }) { amount }
        }
    "#};

    assert_eq!(
        json!({
            "data": {
                "createOneTransaction": {
                    "amount": 1.59283191
                }
            }
        }),
        query_engine.request(query).await
    );

    Ok(())
}
