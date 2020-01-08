use super::*;

/// Represents arbitrary, even nested, expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Any numeric value e.g. floats or ints.
    NumericValue(String, Span),
    /// Any boolean value.
    BooleanValue(String, Span),
    /// Any string value.
    StringValue(String, Span),
    /// A ducktyped string value, used as function return values which can be ducktyped.
    /// Canbe any scalar type, array or function is not possible.
    Any(String, Span),
    /// Any literal constant, basically a string which was not inside "...".
    /// This is used for representing builtin enums.
    ConstantValue(String, Span),
    /// A function with a name and arguments, which is evaluated at client side.
    Function(String, Vec<Expression>, Span),
    /// An array of other values.
    Array(Vec<Expression>, Span),
}

impl Expression {
    pub fn with_lifted_span(&self, offset: usize) -> Expression {
        match self {
            Expression::NumericValue(v, s) => Expression::NumericValue(v.clone(), lift_span(&s, offset)),
            Expression::BooleanValue(v, s) => Expression::BooleanValue(v.clone(), lift_span(&s, offset)),
            Expression::StringValue(v, s) => Expression::StringValue(v.clone(), lift_span(&s, offset)),
            Expression::ConstantValue(v, s) => Expression::ConstantValue(v.clone(), lift_span(&s, offset)),
            Expression::Function(v, a, s) => Expression::Function(
                v.clone(),
                a.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
                lift_span(&s, offset),
            ),
            Expression::Array(v, s) => Expression::Array(
                v.iter().map(|elem| elem.with_lifted_span(offset)).collect(),
                lift_span(&s, offset),
            ),
            Expression::Any(v, s) => Expression::Any(v.clone(), lift_span(&s, offset)),
        }
    }

    pub fn render_to_string(&self) -> String {
        crate::ast::renderer::Renderer::render_value_to_string(self)
    }

    pub fn span(&self) -> Span {
        match &self {
            Self::NumericValue(_, span) => *span,
            Self::BooleanValue(_, span) => *span,
            Self::StringValue(_, span) => *span,
            Self::Any(_, span) => *span,
            Self::ConstantValue(_, span) => *span,
            Self::Function(_, _, span) => *span,
            Self::Array(_, span) => *span,
        }
    }

    pub fn is_env_expression(&self) -> bool {
        match &self {
            Self::Function(name, _, _) => name == "env",
            _ => false,
        }
    }
}

impl ToString for Expression {
    fn to_string(&self) -> String {
        match self {
            Expression::StringValue(x, _) => x.clone(),
            Expression::NumericValue(x, _) => x.clone(),
            Expression::BooleanValue(x, _) => x.clone(),
            Expression::ConstantValue(x, _) => x.clone(),
            Expression::Function(x, _, _) => x.clone(),
            Expression::Array(_, _) => String::from("(array)"),
            Expression::Any(x, _) => x.clone(),
        }
    }
}

impl std::str::FromStr for Expression {
    type Err = pest::error::Error<crate::ast::parser::Rule>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use super::parser::{parse_expression, PrismaDatamodelParser, Rule};
        use pest::Parser;

        // Unwrapping is safe because we know that an expression was parsed.
        let pair = PrismaDatamodelParser::parse(Rule::expression, s)?.next().unwrap();

        Ok(parse_expression(&pair))
    }
}

/// Creates a friendly readable representation for a value's type.
pub fn describe_value_type(val: &Expression) -> &'static str {
    match val {
        Expression::NumericValue(_, _) => "numeric",
        Expression::BooleanValue(_, _) => "boolean",
        Expression::StringValue(_, _) => "string",
        Expression::ConstantValue(_, _) => "literal",
        Expression::Function(_, _, _) => "functional",
        Expression::Array(_, _) => "array",
        Expression::Any(_, _) => "any",
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ast_expression_from_str_works() {
        let expression_str = r##"concatenateStrings(["meow", "woof", "honk"], 8)"##;
        let expr: Expression = expression_str.parse().unwrap();

        let func_arguments = match &expr {
            Expression::Function(name, args, _span) => {
                assert_eq!(name, "concatenateStrings");
                args
            }
            _ => panic!(),
        };

        match func_arguments.get(1) {
            Some(Expression::NumericValue(s, _)) => assert_eq!(s, "8"),
            other => panic!("{:?}", other),
        }

        match func_arguments.get(0) {
            Some(Expression::Array(strings, _)) => {
                let strings = strings
                    .into_iter()
                    .map(|arg| match arg {
                        Expression::StringValue(s, _) => s.as_str(),
                        _ => unreachable!(),
                    })
                    .collect::<Vec<&str>>();

                assert_eq!(strings.as_slice(), &["meow", "woof", "honk"]);
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn ast_expression_from_str_does_not_panic_with_empty_strings() {
        let expression_str = "";
        let expr: Result<Expression, _> = expression_str.parse();
        assert!(expr.is_err());
    }

    #[test]
    fn ast_expression_render_to_string_works() {
        let expression_str = r##"concatenateStrings(["meow", "woof", "honk"], 8)"##;
        let parsed: Expression = expression_str.parse().unwrap();
        assert_eq!(parsed.render_to_string(), expression_str);
    }
}
