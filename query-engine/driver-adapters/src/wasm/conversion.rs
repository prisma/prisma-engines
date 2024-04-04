use crate::conversion::JSArg;

use super::to_js::{serde_serialize, ToJsValue};
use crate::types::Query;
use js_sys::{Array, JsString, Object, Reflect, Uint8Array};
use wasm_bindgen::JsValue;

impl ToJsValue for Query {
    fn to_js_value(&self) -> Result<wasm_bindgen::prelude::JsValue, wasm_bindgen::prelude::JsValue> {
        let object = Object::new();
        let sql = self.sql.to_js_value()?;
        Reflect::set(&object, &JsValue::from(JsString::from("sql")), &sql)?;
        let args = Array::new();
        for arg in &self.args {
            let value = arg.to_js_value()?;
            args.push(&value);
        }
        Reflect::set(&object, &JsValue::from(JsString::from("args")), &args)?;

        Ok(JsValue::from(object))
    }
}

impl ToJsValue for JSArg {
    fn to_js_value(&self) -> Result<wasm_bindgen::prelude::JsValue, wasm_bindgen::prelude::JsValue> {
        match self {
            JSArg::SafeInt(num) => Ok(JsValue::from(*num)),
            JSArg::Value(value) => serde_serialize(value),
            JSArg::Buffer(buf) => {
                let array = Uint8Array::from(buf.as_slice());
                Ok(array.into())
            }
            JSArg::Array(value) => {
                let array = Array::new();
                for arg in value {
                    let js_arg = arg.to_js_value()?;
                    array.push(&js_arg);
                }

                Ok(JsValue::from(array))
            }
        }
    }
}
