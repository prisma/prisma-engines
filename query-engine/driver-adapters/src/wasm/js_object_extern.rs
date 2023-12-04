use js_sys::{JsString, Object as JsObject};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = String, extends = JsObject, is_type_of = JsValue::is_object, typescript_type = "object")]
    pub type JsObjectExtern;

    #[wasm_bindgen(method, catch, structural, indexing_getter)]
    pub fn get(this: &JsObjectExtern, key: JsString) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(static_method_of = JsObjectExtern, js_name = hasOwn)]
    pub fn has_own(this: &JsObjectExtern, key: JsString) -> bool;
}
