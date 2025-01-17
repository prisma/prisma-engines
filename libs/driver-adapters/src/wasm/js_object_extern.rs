// `clippy::empty_docs` is required because of the `wasm-bindgen` crate.
#![allow(clippy::empty_docs)]

use js_sys::{JsString, Object as JsObject};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = String, extends = JsObject, is_type_of = JsValue::is_object, typescript_type = "object")]
    pub type JsObjectExtern;

    // Note: this custom getter allows us to avoid runtime reflection via `js_sys::Reflect`.
    #[wasm_bindgen(method, catch, structural, indexing_getter)]
    pub fn get(this: &JsObjectExtern, key: JsString) -> Result<JsValue, JsValue>;
}
