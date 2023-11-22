use js_sys::JsString;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
extern "C" {
    pub type JsObjectExtern;

    #[wasm_bindgen(method, catch, structural, indexing_getter)]
    pub fn get(this: &JsObjectExtern, key: JsString) -> Result<JsValue, JsValue>;
}
