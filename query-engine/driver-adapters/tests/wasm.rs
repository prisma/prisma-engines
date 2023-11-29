#![cfg(target_os = "wasm32")]
use wasm_bindgen_test::*;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

// Recursive expansion of Deserialize macro
// =========================================
//
// #[doc(hidden)]
// #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
// const _: () = {
//     #[allow(unused_extern_crates, clippy::useless_attribute)]
//     extern crate serde as _serde;
//     #[automatically_derived]
//     impl<'de> _serde::Deserialize<'de> for ColumnType {
//         fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
//         where
//             __D: _serde::Deserializer<'de>,
//         {
//             #[allow(non_camel_case_types)]
//             #[doc(hidden)]
//             enum __Field {
//                 __field0,
//                 __field1,
//             }
//             #[doc(hidden)]
//             struct __FieldVisitor;

//             impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
//                 type Value = __Field;
//                 fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
//                     _serde::__private::Formatter::write_str(__formatter, "variant identifier")
//                 }
//                 fn visit_u64<__E>(self, __value: u64) -> _serde::__private::Result<Self::Value, __E>
//                 where
//                     __E: _serde::de::Error,
//                 {
//                     match __value {
//                         0u64 => _serde::__private::Ok(__Field::__field0),
//                         1u64 => _serde::__private::Ok(__Field::__field1),
//                         _ => _serde::__private::Err(_serde::de::Error::invalid_value(
//                             _serde::de::Unexpected::Unsigned(__value),
//                             &"variant index 0 <= i < 2",
//                         )),
//                     }
//                 }
//                 fn visit_str<__E>(self, __value: &str) -> _serde::__private::Result<Self::Value, __E>
//                 where
//                     __E: _serde::de::Error,
//                 {
//                     match __value {
//                         "Int32" => _serde::__private::Ok(__Field::__field0),
//                         "Int64" => _serde::__private::Ok(__Field::__field1),
//                         _ => _serde::__private::Err(_serde::de::Error::unknown_variant(__value, VARIANTS)),
//                     }
//                 }
//                 fn visit_bytes<__E>(self, __value: &[u8]) -> _serde::__private::Result<Self::Value, __E>
//                 where
//                     __E: _serde::de::Error,
//                 {
//                     match __value {
//                         b"Int32" => _serde::__private::Ok(__Field::__field0),
//                         b"Int64" => _serde::__private::Ok(__Field::__field1),
//                         _ => {
//                             let __value = &_serde::__private::from_utf8_lossy(__value);
//                             _serde::__private::Err(_serde::de::Error::unknown_variant(__value, VARIANTS))
//                         }
//                     }
//                 }
//             }
//             impl<'de> _serde::Deserialize<'de> for __Field {
//                 #[inline]
//                 fn deserialize<__D>(__deserializer: __D) -> _serde::__private::Result<Self, __D::Error>
//                 where
//                     __D: _serde::Deserializer<'de>,
//                 {
//                     _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
//                 }
//             }
//             #[doc(hidden)]
//             struct __Visitor<'de> {
//                 marker: _serde::__private::PhantomData<ColumnType>,
//                 lifetime: _serde::__private::PhantomData<&'de ()>,
//             }
//             impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
//                 type Value = ColumnType;
//                 fn expecting(&self, __formatter: &mut _serde::__private::Formatter) -> _serde::__private::fmt::Result {
//                     _serde::__private::Formatter::write_str(__formatter, "enum ColumnType")
//                 }
//                 fn visit_enum<__A>(self, __data: __A) -> _serde::__private::Result<Self::Value, __A::Error>
//                 where
//                     __A: _serde::de::EnumAccess<'de>,
//                 {
//                     match _serde::de::EnumAccess::variant(__data)? {
//                         (__Field::__field0, __variant) => {
//                             _serde::de::VariantAccess::unit_variant(__variant)?;
//                             _serde::__private::Ok(ColumnType::Int32)
//                         }
//                         (__Field::__field1, __variant) => {
//                             _serde::de::VariantAccess::unit_variant(__variant)?;
//                             _serde::__private::Ok(ColumnType::Int64)
//                         }
//                     }
//                 }
//             }
//             #[doc(hidden)]
//             const VARIANTS: &'static [&'static str] = &["Int32", "Int64"];
//             _serde::Deserializer::deserialize_enum(
//                 __deserializer,
//                 "ColumnType",
//                 VARIANTS,
//                 __Visitor {
//                     marker: _serde::__private::PhantomData::<ColumnType>,
//                     lifetime: _serde::__private::PhantomData,
//                 },
//             )
//         }
//     }
// };
//
//
// Recursive expansion of Tsify macro
// ===================================
//
// #[automatically_derived]
// const _: () = {
//     extern crate serde as _serde;
//     use tsify::Tsify;
//     use wasm_bindgen::{
//         convert::{FromWasmAbi, IntoWasmAbi, OptionFromWasmAbi, OptionIntoWasmAbi},
//         describe::WasmDescribe,
//         prelude::*,
//     };
//     #[wasm_bindgen]
//     extern "C" {
//         #[wasm_bindgen(typescript_type = "ColumnType")]
//         pub type JsType;
//     }
//     impl Tsify for ColumnType {
//         type JsType = JsType;
//         const DECL: &'static str = "export type ColumnType = \"Int32\" | \"Int64\";";
//     }
//     #[wasm_bindgen(typescript_custom_section)]
//     const TS_APPEND_CONTENT: &'static str = "export type ColumnType = \"Int32\" | \"Int64\";";
//     impl WasmDescribe for ColumnType {
//         #[inline]
//         fn describe() {
//             <Self as Tsify>::JsType::describe()
//         }
//     }
//     impl IntoWasmAbi for ColumnType
//     where
//         Self: _serde::Serialize,
//     {
//         type Abi = <JsType as IntoWasmAbi>::Abi;
//         #[inline]
//         fn into_abi(self) -> Self::Abi {
//             self.into_js().unwrap_throw().into_abi()
//         }
//     }
//     impl OptionIntoWasmAbi for ColumnType
//     where
//         Self: _serde::Serialize,
//     {
//         #[inline]
//         fn none() -> Self::Abi {
//             <JsType as OptionIntoWasmAbi>::none()
//         }
//     }
//     impl FromWasmAbi for ColumnType
//     where
//         Self: _serde::de::DeserializeOwned,
//     {
//         type Abi = <JsType as FromWasmAbi>::Abi;
//         #[inline]
//         unsafe fn from_abi(js: Self::Abi) -> Self {
//             let result = Self::from_js(&JsType::from_abi(js));
//             if let Err(err) = result {
//                 wasm_bindgen::throw_str(err.to_string().as_ref());
//             }
//             result.unwrap_throw()
//         }
//     }
//     impl OptionFromWasmAbi for ColumnType
//     where
//         Self: _serde::de::DeserializeOwned,
//     {
//         #[inline]
//         fn is_none(js: &Self::Abi) -> bool {
//             <JsType as OptionFromWasmAbi>::is_none(js)
//         }
//     }
// };
#[derive(Clone, Copy, Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
pub enum ColumnType {
    Int32 = 0,
    Int64 = 1,
}

#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
struct ColumnTypeWrapper {
    column_type: ColumnType,
}

// Recursive expansion of Deserialize_repr macro
// ==============================================
//
// impl<'de> serde::Deserialize<'de> for ColumnTypeWasmBindgen {
//   #[allow(clippy::use_self)]
//   fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
//   where
//       D: serde::Deserializer<'de>,
//   {
//       #[allow(non_camel_case_types)]
//       struct discriminant;

//       #[allow(non_upper_case_globals)]
//       impl discriminant {
//           const Int32: u8 = ColumnTypeWasmBindgen::Int32 as u8;
//           const Int64: u8 = ColumnTypeWasmBindgen::Int64 as u8;
//       }
//       match <u8 as serde::Deserialize>::deserialize(deserializer)? {
//           discriminant::Int32 => ::core::result::Result::Ok(ColumnTypeWasmBindgen::Int32),
//           discriminant::Int64 => ::core::result::Result::Ok(ColumnTypeWasmBindgen::Int64),
//           other => ::core::result::Result::Err(serde::de::Error::custom(format_args!(
//               "invalid value: {}, expected {} or {}",
//               other,
//               discriminant::Int32,
//               discriminant::Int64
//           ))),
//       }
//   }
// }
#[derive(Debug, Deserialize_repr, Tsify)]
#[tsify(from_wasm_abi)]
#[repr(u8)]
pub enum ColumnTypeWasmBindgen {
    // #[serde(rename = "0")]
    Int32 = 0,

    // #[serde(rename = "1")]
    Int64 = 1,
}

#[wasm_bindgen_test]
fn column_type_test() {
    // Example deserialization code
    let json_data = r#"0"#;
    let column_type = serde_json::from_str::<u8>(&json_data).unwrap();
    let column_type = serde_json::from_str::<i8>(&json_data).unwrap();
    let column_type = serde_json::from_str::<u16>(&json_data).unwrap();
    let column_type = serde_json::from_str::<i16>(&json_data).unwrap();
    let column_type = serde_json::from_str::<u32>(&json_data).unwrap();
    let column_type = serde_json::from_str::<i32>(&json_data).unwrap();
    let column_type = serde_json::from_str::<u64>(&json_data).unwrap();
    let column_type = serde_json::from_str::<i64>(&json_data).unwrap();

    // let json_data = "\"0\"";
    let column_type = serde_json::from_str::<ColumnTypeWasmBindgen>(&json_data).unwrap();
}

// #[wasm_bindgen_test]
// fn column_type_test() {
//     // Example deserialization code
//     let json_data = r#"{ "columnType": 0 }"#;
//     let column_type_wrapper = serde_json::from_str::<ColumnTypeWrapper>(json_data);

//     panic!("{:?}", column_type_wrapper);
// }
