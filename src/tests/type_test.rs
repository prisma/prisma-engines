use crate::single::Quaint;

#[async_trait::async_trait]
pub trait TypeTest {
    async fn new() -> crate::Result<Self>
    where
        Self: Sized;

    async fn create_table(&mut self, r#type: &str) -> crate::Result<String>;

    fn conn(&self) -> &Quaint;
}
