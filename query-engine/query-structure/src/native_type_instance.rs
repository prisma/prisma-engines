use psl::datamodel_connector::{NativeTypeInstance as PslNativeTypeInstance, ValidatedConnector};

/// Represents an instance of a native type declared in the Prisma schema.
#[derive(Clone)]
pub struct NativeTypeInstance {
    pub native_type: PslNativeTypeInstance,
    pub connector: &'static dyn ValidatedConnector,
}

impl NativeTypeInstance {
    pub fn new(native_type: PslNativeTypeInstance, connector: &'static dyn ValidatedConnector) -> Self {
        NativeTypeInstance { native_type, connector }
    }

    pub fn deserialize_native_type<T: std::any::Any>(&self) -> &T {
        self.native_type.downcast_ref()
    }

    pub fn name(&self) -> &'static str {
        self.connector.native_type_to_parts(&self.native_type).0
    }

    pub fn args(&self) -> Vec<String> {
        self.connector.native_type_to_parts(&self.native_type).1
    }
}
