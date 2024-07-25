use quaint::Value;

pub(crate) const SIMPLE_SCHEMA: &str = r#"
model model {
    int     Int     @id
    string  String
    bigint  BigInt
    float   Float
    bytes   Bytes
    bool    Boolean
    dt      DateTime
}"#;

pub(crate) const ENUM_SCHEMA: &str = r#"
model model {
    id     Int     @id
    enum    MyFancyEnum
}

enum MyFancyEnum {
    A
    B
    C
}
"#;

pub(crate) fn typ_to_value(typ: &str) -> Value<'static> {
    match typ {
        "string" => Value::text("hello"),
        "int" => Value::int32(i8::MAX),
        "bigint" => Value::int64(i8::MAX),
        "float" => Value::float(f32::EPSILON),
        "double" => Value::double(f64::EPSILON),
        "bytes" => Value::bytes("hello".as_bytes()),
        "bool" => Value::boolean(false),
        "datetime" => Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z")
                .unwrap()
                .into(),
        ),
        _ => unimplemented!(),
    }
}
