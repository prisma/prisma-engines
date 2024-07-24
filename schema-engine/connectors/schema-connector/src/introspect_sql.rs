#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlContext {
    pub queries: Vec<IntrospectSqlQueryInput>,
    pub force: bool,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlQueryInput {
    pub name: String,
    pub source: String,
}

#[allow(missing_docs)]
pub struct IntrospectSqlResult {
    pub queries: Vec<IntrospectSqlQueryOutput>,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlQueryOutput {
    pub documentation: String,
    pub name: String,
    pub parameters: Vec<IntrospectSqlQueryParameterOutput>,
    pub result_columns: Vec<IntrospectSqlQueryColumnOutput>,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlQueryParameterOutput {
    pub documentation: String,
    pub name: String,
    pub typ: String,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlQueryColumnOutput {
    pub name: String,
    pub typ: String,
}

impl From<quaint::connector::ParsedRawItem> for IntrospectSqlQueryParameterOutput {
    fn from(item: quaint::connector::ParsedRawItem) -> Self {
        Self {
            name: item.name,
            documentation: String::new(),
            typ: item.typ.to_string(),
        }
    }
}

impl From<quaint::connector::ParsedRawItem> for IntrospectSqlQueryColumnOutput {
    fn from(item: quaint::connector::ParsedRawItem) -> Self {
        Self {
            name: item.name,
            typ: item.typ.to_string(),
        }
    }
}
