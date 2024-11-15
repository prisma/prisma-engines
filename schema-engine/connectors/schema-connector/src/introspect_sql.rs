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
    pub name: String,
    pub source: String,
    pub documentation: Option<String>,
    pub parameters: Vec<IntrospectSqlQueryParameterOutput>,
    pub result_columns: Vec<IntrospectSqlQueryColumnOutput>,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlQueryParameterOutput {
    pub documentation: Option<String>,
    pub name: String,
    pub typ: String,
    pub nullable: bool,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct IntrospectSqlQueryColumnOutput {
    pub name: String,
    pub typ: String,
    pub nullable: bool,
}

impl From<quaint::connector::DescribedColumn> for IntrospectSqlQueryColumnOutput {
    fn from(item: quaint::connector::DescribedColumn) -> Self {
        let nullable_override = parse_nullability_override(&item.name);

        Self {
            name: item.name,
            typ: item.enum_name.unwrap_or_else(|| item.typ.to_string()),
            nullable: nullable_override.unwrap_or(item.nullable),
        }
    }
}

fn parse_nullability_override(column_name: &str) -> Option<bool> {
    if column_name.ends_with('?') {
        Some(true)
    } else if column_name.ends_with('!') {
        Some(false)
    } else {
        None
    }
}
