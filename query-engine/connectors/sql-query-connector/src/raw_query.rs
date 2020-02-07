use prisma_value::PrismaValue;
use quaint::ast::ParameterizedValue;

pub struct RawQuery<'a> {
    query: String,
    parameters: Vec<ParameterizedValue<'a>>,
}

#[allow(dead_code)]
impl<'a> RawQuery<'a> {
    pub fn new(query: String, parameters: Vec<PrismaValue>) -> Self {
        let parameters = parameters.into_iter().map(ParameterizedValue::from).collect();
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

    pub fn parameters(&self) -> &[ParameterizedValue<'a>] {
        self.parameters.as_slice()
    }
}
