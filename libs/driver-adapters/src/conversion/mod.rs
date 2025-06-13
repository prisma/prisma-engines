pub(crate) mod js_arg;
pub(crate) mod js_arg_type;
pub(crate) mod js_to_quaint;

#[cfg(feature = "mssql")]
pub(crate) mod mssql;
#[cfg(feature = "mysql")]
pub(crate) mod mysql;
#[cfg(feature = "postgresql")]
pub(crate) mod postgres;
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;

pub use js_arg::JSArg;
pub use js_arg_type::{value_to_js_arg_type, JSArgType};

/// A wrapper around `Option` that indicates that `None` should be treated as
/// `undefined` in JavaScript.
pub struct MaybeDefined<T>(pub Option<T>);

impl<V> From<Option<V>> for MaybeDefined<V> {
    fn from(value: Option<V>) -> Self {
        Self(value)
    }
}
