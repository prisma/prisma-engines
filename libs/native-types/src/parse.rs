use crate::NativeTypeError;

pub trait ParseTypeParameter<T> {
    fn as_param(&self, context: &str) -> crate::Result<T>;
}

impl ParseTypeParameter<u8> for &str {
    fn as_param(&self, context: &str) -> crate::Result<u8>
    where
        Self: Sized,
    {
        self.parse()
            .map_err(|_| NativeTypeError::invalid_parameter(self, "u8", context))
    }
}

impl ParseTypeParameter<u16> for &str {
    fn as_param(&self, context: &str) -> crate::Result<u16>
    where
        Self: Sized,
    {
        self.parse()
            .map_err(|_| NativeTypeError::invalid_parameter(self, "u16", context))
    }
}
