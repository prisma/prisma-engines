use quaint::prelude::Queryable;
use std::sync::Arc;

// TODO: implement registry for client drivers, rather than a global variable,
// this would require the register_driver and registered_js_driver functions to
// receive an identifier for the specific driver
static QUERYABLE: once_cell::sync::OnceCell<Arc<dyn Queryable>> = once_cell::sync::OnceCell::new();

pub fn registered_driver() -> Option<&'static Arc<dyn Queryable>> {
    QUERYABLE.get()
}

pub fn register_driver(driver: Arc<dyn Queryable>) {
    if QUERYABLE.set(driver).is_err() {
        panic!("Cannot register driver twice");
    }
}
