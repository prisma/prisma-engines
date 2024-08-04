pub trait StringCallback {
    fn call(&self, message: String) -> Result<(), String>;
}
