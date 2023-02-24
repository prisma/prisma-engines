use psl_core::datamodel_connector::{Connector, NativeTypeInstance as PslNativeTypeInstance};
use std::fmt;

/// Represents an instance of a native type declared in the Prisma schema.
#[derive(Clone)]
pub struct NativeTypeInstance {
    pub native_type: PslNativeTypeInstance,
    pub connector: &'static dyn Connector,
}

impl fmt::Debug for NativeTypeInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.connector.native_type_to_string(&self.native_type))
    }
}

impl PartialEq for NativeTypeInstance {
    fn eq(&self, other: &Self) -> bool {
        self.connector.native_type_to_parts(&self.native_type)
            == other.connector.native_type_to_parts(&other.native_type)
    }
}

impl NativeTypeInstance {
    pub fn new(native_type: PslNativeTypeInstance, connector: &'static dyn Connector) -> Self {
        NativeTypeInstance { native_type, connector }
    }

    pub fn inner(&self) -> &PslNativeTypeInstance {
        &self.native_type
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
