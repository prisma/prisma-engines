use datamodel_connector::{
    scalars::ScalarType, Connector, ConnectorCapability, NativeTypeConstructor, NativeTypeInstance,
};
use native_types::{NativeType, PostgresType};

const BIG_INT_TYPE_NAME: &str = "BigInt";
const VARCHAR_TYPE_NAME: &str = "VarChar";

pub struct PostgresDatamodelConnector {
    capabilities: Vec<ConnectorCapability>,
    constructors: Vec<NativeTypeConstructor>,
}

impl PostgresDatamodelConnector {
    pub fn new() -> PostgresDatamodelConnector {
        let capabilities = vec![
            ConnectorCapability::ScalarLists,
            ConnectorCapability::Enums,
            ConnectorCapability::Json,
        ];

        let bigint = NativeTypeConstructor::without_args(BIG_INT_TYPE_NAME, ScalarType::Int);
        let varchar = NativeTypeConstructor::with_args(VARCHAR_TYPE_NAME, 1, ScalarType::String);
        let constructors = vec![varchar, bigint];

        PostgresDatamodelConnector {
            capabilities,
            constructors,
        }
    }
}

impl Connector for PostgresDatamodelConnector {
    fn capabilities(&self) -> &Vec<ConnectorCapability> {
        &self.capabilities
    }

    fn available_native_type_constructors(&self) -> &Vec<NativeTypeConstructor> {
        &self.constructors
    }

    fn parse_native_type(&self, name: &str, args: Vec<u32>) -> Option<NativeTypeInstance> {
        let constructor = self.find_native_type_constructor(name);
        let native_type = match name {
            BIG_INT_TYPE_NAME => PostgresType::BigInt,
            VARCHAR_TYPE_NAME => {
                let length = *args.first().unwrap();
                PostgresType::VarChar(length)
            }
            _ => unreachable!("This code is unreachable as the core must guarantee to just call with known names."),
        };

        Some(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type))
    }

    fn introspect_native_type(&self, native_type: Box<dyn NativeType>) -> Option<NativeTypeInstance> {
        let native_type: PostgresType = serde_json::from_value(native_type.to_json()).unwrap();
        let (constructor_name, args) = match native_type {
            PostgresType::BigInt => (BIG_INT_TYPE_NAME, vec![]),
            PostgresType::VarChar(x) => (VARCHAR_TYPE_NAME, vec![x]),
            _ => todo!("This match must be exhaustive"),
        };

        let constructor = self.find_native_type_constructor(constructor_name);

        Some(NativeTypeInstance::new(constructor.name.as_str(), args, &native_type))
    }
}
