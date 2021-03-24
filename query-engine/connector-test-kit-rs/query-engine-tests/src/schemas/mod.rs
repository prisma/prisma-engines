// Wild idea: validate schemas at compile time
pub fn some_common_schema() -> String {
    "model C {
            id Int @id
            field String?
        }"
    .to_owned()
}
