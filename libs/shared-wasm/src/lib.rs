use serde_wasm_bindgen::Serializer;

// - `serialize_missing_as_null` is required to make sure that "empty" values (e.g., `None` and `()`)
//   are serialized as `null` and not `undefined`.
//   This is due to certain drivers (e.g., LibSQL) not supporting `undefined` values.
// - `serialize_maps_as_objects` is required because the client always expects objects for
//    all Rust map types.
pub const RESPONSE_SERIALIZER: Serializer = Serializer::new()
    .serialize_maps_as_objects(true)
    .serialize_missing_as_null(true);

// - `serialize_large_number_types_as_bigints` is required to allow reading bigints from Prisma Client.
pub const RESPONSE_WITH_BIGINT_SERIALIZER: Serializer =
    RESPONSE_SERIALIZER.serialize_large_number_types_as_bigints(true);
