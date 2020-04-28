use prisma_value::PrismaValue;
use quaint::ast::Value;

pub struct RawQuery<'a> {
    query: String,
    parameters: Vec<Value<'a>>,
}

#[allow(dead_code)]
impl<'a> RawQuery<'a> {
    pub fn new(query: String, parameters: Vec<PrismaValue>) -> Self {
        let parameters = parameters.into_iter().map(Value::from).collect();
        let query = query.trim().to_string();

        Self { query, parameters }
    }

    pub fn is_select(&self) -> bool {
        self.query
            .split(" ")
            .next()
            .map(|t| t.to_uppercase().trim() == "SELECT")
            .unwrap_or(false)
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn parameters(&self) -> &[Value<'a>] {
        self.parameters.as_slice()
    }
}
