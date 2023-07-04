use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
pub struct Context {
    store: HashMap<(String, TypeId), Box<dyn Any>>,
}

impl Context {
    pub fn concurrent(self) -> Arc<Mutex<Context>> {
        Arc::new(Mutex::new(self))
    }

    pub fn insert<T: Any>(&mut self, key: &str, value: T) {
        self.store.insert((key.to_owned(), TypeId::of::<T>()), Box::new(value));
    }

    pub fn get<T: Any>(&self, key: &str) -> Option<&T> {
        self.store
            .get(&(key.to_owned(), TypeId::of::<T>()))
            .map(|v| v.downcast_ref::<T>().unwrap())
    }

    pub fn get_mut<T: Any>(&mut self, key: &str) -> Option<&mut T> {
        self.store
            .get_mut(&(key.to_owned(), TypeId::of::<T>()))
            .map(|v| v.downcast_mut::<T>().unwrap())
    }

    pub fn remove<T: Any>(&mut self, key: &str) -> Option<T> {
        self.store
            .remove(&(key.to_owned(), TypeId::of::<T>()))
            .map(|v| *v.downcast::<T>().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::Context;

    #[test]
    fn set_and_retrieve() {
        let mut ctx: Context = Context::default();
        ctx.insert("foo", 42 as u32);

        let val: u32 = *ctx.get("foo").unwrap();
        assert_eq!(val, 42 as u32)
    }

    #[test]
    fn concurrent() {
        let mut ctx: Context = Context::default();
        ctx.insert("foo", 42 as u32);

        assert_eq!(42 as u32, *ctx.get::<u32>("foo").unwrap());
        assert_eq!(None, ctx.get::<u32>("bar"));

        let safe_context = ctx.concurrent();
        let mut ctx = safe_context.lock().unwrap();
        ctx.insert("bar", 32 as u32);
        assert_eq!(32 as u32, *ctx.get::<u32>("bar").unwrap());
        assert_eq!(42 as u32, *ctx.get::<u32>("foo").unwrap());
    }

    #[test]
    fn get_mut() {
        let mut ctx: Context = Context::default();
        ctx.insert("foo", 42 as u32);

        let val: &mut u32 = ctx.get_mut("foo").unwrap();
        *val = 32 as u32;
        assert_eq!(32 as u32, *ctx.get::<u32>("foo").unwrap());
    }

    #[test]
    fn remove() {
        let mut ctx: Context = Context::default();
        ctx.insert("foo", 42 as u32);

        let val: u32 = ctx.remove("foo").unwrap();
        assert_eq!(42 as u32, val);
        assert_eq!(None, ctx.get::<u32>("foo"));
    }
}
