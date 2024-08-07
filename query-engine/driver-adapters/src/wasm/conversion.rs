use crate::conversion::{JSArg, JSArgType};

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

        let arg_types = Array::new();
        for arg_type_opt in &self.arg_types {
            // We need to unpack `Option<JSArgType>` in place to avoid the "conflicting implementation for `std::option::Option<JSArgType>`" compilation error
            // when "impl ToJsValue for Option<JSArgType>" exists, or the "method `to_js_value` exists for enum `Option<JSArgType>`, but its trait bounds were not satisfied"
            // compilation error otherwise.
            let value = match arg_type_opt {
                Some(arg_type) => arg_type.to_js_value()?,
                None => JsValue::null(),
            };
            arg_types.push(&value);
        }
        Reflect::set(&object, &JsValue::from(JsString::from("argTypes")), &arg_types)?;

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

impl ToJsValue for JSArgType {
    fn to_js_value(&self) -> Result<wasm_bindgen::prelude::JsValue, wasm_bindgen::prelude::JsValue> {
        Ok(JsValue::from(self.to_string()))
    }
}
