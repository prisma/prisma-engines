use quaint::prelude::BoxedQueryable;

// TODO: implement registry for client drivers, rather than a global variable,
// this would require the register_driver and registered_js_driver functions to
// receive an identifier for the specific driver
static QUERYABLE: once_cell::sync::OnceCell<Box<dyn BoxedQueryable>> = once_cell::sync::OnceCell::new();

pub fn registered_driver() -> Option<&'static dyn BoxedQueryable> {
    QUERYABLE.get().map(|q| q.as_ref())
}

pub fn register_driver(driver: Box<dyn BoxedQueryable>) {
    if QUERYABLE.set(driver).is_err() {
        panic!("Cannot register driver twice");
    }
}
