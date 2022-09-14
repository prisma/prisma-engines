use std::fmt;

#[derive(Debug)]
pub struct NativeTypeInstance {
    pub name: String,
    pub args: Vec<String>,
    pub serialized_native_type: serde_json::Value,
}

impl fmt::Display for NativeTypeInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;

        if !self.args.is_empty() {
            f.write_str("(")?;
            f.write_str(&self.args.join(","))?;
            f.write_str(")")?;
        }

        Ok(())
    }
}

impl NativeTypeInstance {
    pub fn new(name: &str, args: Vec<String>, serialized_native_type: serde_json::Value) -> Self {
        NativeTypeInstance {
            name: name.to_string(),
            args,
            serialized_native_type,
        }
    }
}
