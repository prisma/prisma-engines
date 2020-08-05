use datamodel_connector::{ConnectorCapability, DeclarativeConnector, FieldTypeConstructor};

pub fn new() -> DeclarativeConnector {
    let capabilities = vec![
        ConnectorCapability::RelationsOverNonUniqueCriteria,
        ConnectorCapability::Enums,
        ConnectorCapability::Json,
        ConnectorCapability::MultipleIndexesWithSameName,
    ];

    let constructors: Vec<FieldTypeConstructor> = vec![];

    DeclarativeConnector {
        capabilities,
        field_type_constructors: constructors,
    }
}
