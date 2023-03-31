use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::{prelude::wasm_bindgen, JsError, JsValue};

use js_sys::Promise as JsPromise;
use wasm_bindgen_futures::JsFuture;

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
  }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Encapsulates a set of results and their respective column names.
#[derive(Debug, Default, Serialize, Clone, Tsify)]
#[tsify(into_wasm_abi)]
pub struct ResultSet {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
}

#[wasm_bindgen(module = "/queryable.js")]
extern "C" {
    pub type Queryable;

    #[wasm_bindgen(constructor)]
    fn new() -> Queryable;

    #[wasm_bindgen(method)]
    fn connect(this: &Queryable) -> JsValue;

    #[wasm_bindgen(method)]
    fn disconnect(this: &Queryable) -> JsValue;

    #[wasm_bindgen(method)]
    fn query(this: &Queryable, query: &str) -> JsValue;

    #[wasm_bindgen(method)]
    fn query_raw(this: &Queryable, query: &str, params: Vec<JsValue>) -> JsValue;

    #[wasm_bindgen(method)]
    fn is_healthy(this: &Queryable) -> JsValue;
}

#[wasm_bindgen]
pub async fn find_many(connector: Queryable, model_name: &str) -> Result<JsValue, JsValue> {
    console_log!("[Wasm] calling find_many");
    let promiseable = connector.query(&format!("SELECT * FROM {model_name}"));
    let result = to_future(promiseable).await?;
    console_log!("[Wasm] promise resolved");
    Ok(result)
}

#[derive(Debug, Deserialize, Clone, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(transparent)]
pub struct CreateParams(BTreeMap<String, String>);

#[wasm_bindgen]
pub async fn create(connector: Queryable, model_name: &str, create_params: CreateParams) -> Result<JsValue, JsValue> {
    console_log!("[Wasm] calling create");
    let (fields, values): (Vec<String>, Vec<String>) = create_params
        .0
        .into_iter()
        .map(|(field, value)| (format!("'{field}'"), format!("'{value}'")))
        .unzip();
    let fields = fields.join(", ");
    let values = values.join(", ");
    let promiseable = connector.query(&format!("INSERT INTO ({fields}) VALUES ({values})"));
    let result = to_future(promiseable).await?;
    console_log!("[Wasm] promise resolved");
    Ok(result)
}

async fn to_future(promiseable: JsValue) -> Result<JsValue, JsValue> {
    console_log!("[Wasm] converting promiseable -> promise");
    let promise = JsPromise::from(promiseable);
    console_log!("[Wasm] converting promise -> future");
    let future = JsFuture::from(promise);
    let result = future.await;
    result
}
