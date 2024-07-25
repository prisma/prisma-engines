use std::any::Any;

use psl::{datamodel_connector::NativeTypeInstance, parser_database::ScalarType};
use sql_migration_tests::test_api::TestApi;

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

pub(crate) fn render_scalar_type_datamodel(datasource: &str, prisma_type: ScalarType) -> String {
    let prisma_type = prisma_type.as_str();

    format!(
        r#"
        {datasource}

        model test {{
            id Int @id @default(autoincrement())
            field {prisma_type}
        }}
    "#
    )
}

pub(crate) fn render_native_type_datamodel<T: Any + Send + Sync + 'static>(
    api: &TestApi,
    datasource: &str,
    nt_parts: (&str, Vec<String>),
    nt: T,
) -> String {
    let (nt_name, rest) = nt_parts;
    let args = if rest.is_empty() {
        "".to_string()
    } else {
        format!("({})", rest.join(","))
    };

    let instance = NativeTypeInstance::new::<T>(nt);
    let prisma_type = api.connector.scalar_type_for_native_type(&instance).as_str();

    format!(
        r#"
        {datasource}

        model test {{
            id Int @id @default(autoincrement())
            field {prisma_type} @db.{nt_name}{args}
        }}
    "#
    )
}

macro_rules! test_scalar_types {
    (
        $tag:ident;

        $(
            $test_name:ident($st:expr) => ($ct_input:ident, $ct_output:ident),
        )*
    ) => {
            $(
                paste::paste! {
                    #[test_connector(tags($tag))]
                    fn [<$test_name _ $tag:lower>](api: TestApi) {

                        let dm = render_scalar_type_datamodel(DATASOURCE, $st);

                        api.schema_push(&dm).send();

                        api.introspect_sql("test_1", "INSERT INTO test (field) VALUES (?);")
                            .send_sync()
                            .expect_param_type(0, ColumnType::$ct_input);

                        api.introspect_sql("test_2", "SELECT field FROM test;")
                            .send_sync()
                            .expect_column_type(0, ColumnType::$ct_output);
                    }
                }
            )*
    };
}

pub(crate) use test_scalar_types;
