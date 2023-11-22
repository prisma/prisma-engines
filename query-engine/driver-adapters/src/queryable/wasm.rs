use crate::wasm::proxy::{CommonProxy, DriverProxy};
use crate::{JsObjectExtern, JsQueryable};
use psl::datamodel_connector::Flavour;
use wasm_bindgen::prelude::wasm_bindgen;

/// A JsQueryable adapts a Proxy to implement quaint's Queryable interface. It has the
/// responsibility of transforming inputs and outputs of `query` and `execute` methods from quaint
/// types to types that can be translated into javascript and viceversa. This is to let the rest of
/// the query engine work as if it was using quaint itself. The aforementioned transformations are:
///
/// Transforming a `quaint::ast::Query` into SQL by visiting it for the specific flavour of SQL
/// expected by the client connector. (eg. using the mysql visitor for the Planetscale client
/// connector)
///
/// Transforming a `JSResultSet` (what client connectors implemented in javascript provide)
/// into a `quaint::connector::result_set::ResultSet`. A quaint `ResultSet` is basically a vector
/// of `quaint::Value` but said type is a tagged enum, with non-unit variants that cannot be converted to javascript as is.
#[wasm_bindgen(getter_with_clone)]
#[derive(Default)]
pub(crate) struct JsBaseQueryable {
    pub(crate) proxy: CommonProxy,
    pub flavour: Flavour,
}

pub fn from_wasm(driver: JsObjectExtern) -> JsQueryable {
    let common = CommonProxy::new(&driver).unwrap();
    let driver_proxy = DriverProxy::new(&driver).unwrap();

    JsQueryable {
        inner: JsBaseQueryable::new(common),
        driver_proxy,
    }
}
