use super::{scalars::ScalarType, Connector, ConnectorCapability, ScalarFieldType};
use native_types::NativeType;

pub struct DeclarativeConnector {
    pub capabilities: Vec<ConnectorCapability>,
    pub field_type_constructors: Vec<FieldTypeConstructor>,
}

impl Connector for DeclarativeConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn calculate_native_type(&self, name: &str, args: Vec<u32>) -> Option<ScalarFieldType> {
        self.get_field_type_constructor(&name).map(|constructor| {
            let native_type = (constructor.native_type_fn)(args);

            ScalarFieldType::new(name, constructor.prisma_type, native_type.as_ref())
        })
    }
}

impl DeclarativeConnector {
    fn get_field_type_constructor(&self, name: &str) -> Option<&FieldTypeConstructor> {
        self.field_type_constructors.iter().find(|rt| &rt.name == name)
    }
}

pub struct FieldTypeConstructor {
    name: String,
    _number_of_args: usize,
    prisma_type: ScalarType,
    native_type_fn: Box<dyn Fn(Vec<u32>) -> Box<dyn NativeType> + std::marker::Sync + std::marker::Send>,
}

impl FieldTypeConstructor {
    pub fn without_args(
        name: &str,
        prisma_type: ScalarType,
        native_type_fn: Box<dyn Fn() -> Box<dyn NativeType> + std::marker::Sync + std::marker::Send>,
    ) -> FieldTypeConstructor {
        FieldTypeConstructor {
            name: name.to_string(),
            _number_of_args: 0,
            prisma_type,
            native_type_fn: Box::new(move |_| native_type_fn()),
        }
    }

    pub fn with_args(
        name: &str,
        number_of_args: usize,
        prisma_type: ScalarType,
        native_type_fn: Box<dyn Fn(Vec<u32>) -> Box<dyn NativeType> + std::marker::Sync + std::marker::Send>,
    ) -> FieldTypeConstructor {
        FieldTypeConstructor {
            name: name.to_string(),
            _number_of_args: number_of_args,
            prisma_type,
            native_type_fn,
        }
    }
}
