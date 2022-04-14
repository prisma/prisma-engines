#![allow(clippy::unused_unit)]

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn get_dmmf(schema: String) -> String {
    dmmf::dmmf_json_from_schema(&schema)
}
