use datamodel_connector::{DeclarativeConnector, FieldTypeConstructor};

pub fn new() -> DeclarativeConnector {
    let capabilities = vec![];
    let constructors: Vec<FieldTypeConstructor> = vec![];

    DeclarativeConnector {
        capabilities,
        field_type_constructors: constructors,
    }
}
