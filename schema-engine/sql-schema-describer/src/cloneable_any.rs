use std::any::Any;

pub trait CloneableAny: Any + Send + Sync {
    fn clone_box(&self) -> Box<dyn CloneableAny + Send + Sync>;
}

impl<T> CloneableAny for T
where
    T: Any + Send + Sync + Clone + 'static,
{
    fn clone_box(&self) -> Box<dyn CloneableAny + Send + Sync> {
        Box::new(self.clone())
    }
}

impl dyn CloneableAny + Send + Sync {
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }
}
