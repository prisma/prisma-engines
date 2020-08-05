use datamodel_connector::{scalars::ScalarType, ConnectorCapability, DeclarativeConnector, FieldTypeConstructor};
use native_types::PostgresType;

pub fn new() -> DeclarativeConnector {
    let capabilities = vec![
        ConnectorCapability::ScalarLists,
        ConnectorCapability::Enums,
        ConnectorCapability::Json,
    ];

    let varchar = FieldTypeConstructor::with_args(
        "VarChar",
        1,
        ScalarType::String,
        Box::new(|args| {
            let first_arg = *args.first().unwrap();
            Box::new(PostgresType::VarChar(first_arg))
        }),
    );

    let bigint =
        FieldTypeConstructor::without_args("BigInt", ScalarType::Int, Box::new(|| Box::new(PostgresType::BigInt)));

    let constructors = vec![varchar, bigint];

    DeclarativeConnector {
        capabilities,
        field_type_constructors: constructors,
    }
}
