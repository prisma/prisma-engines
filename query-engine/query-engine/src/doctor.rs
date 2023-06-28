use request_handlers::RequestBody;

pub fn to_prisma_query(b: &RequestBody) -> String {
    serde_json::to_string_pretty(&b).unwrap()
}
