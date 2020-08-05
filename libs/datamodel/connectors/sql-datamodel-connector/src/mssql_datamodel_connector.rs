use datamodel_connector::{ConnectorCapability, DeclarativeConnector, FieldTypeConstructor};

pub fn new() -> DeclarativeConnector {
    let capabilities: Vec<ConnectorCapability> = vec![];
    let constructors: Vec<FieldTypeConstructor> = vec![];

    DeclarativeConnector {
        capabilities,
        field_type_constructors: constructors,
    }
}
