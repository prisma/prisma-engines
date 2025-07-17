use super::to_js::{ToJsValue, serde_serialize};
use crate::conversion::{JSArg, JSArgType, MaybeDefined};
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
        for arg_type in &self.arg_types {
            arg_types.push(&arg_type.to_js_value()?);
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

impl<V: ToJsValue> ToJsValue for MaybeDefined<V> {
    fn to_js_value(&self) -> Result<wasm_bindgen::prelude::JsValue, wasm_bindgen::prelude::JsValue> {
        match &self.0 {
            Some(value) => value.to_js_value(),
            None => Ok(JsValue::UNDEFINED),
        }
    }
}
