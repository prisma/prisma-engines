use super::{Constant, ConstantNameValidationError, Text, Value};
use std::{borrow::Cow, fmt};

/// Represents a function parameter in the PSL.
#[derive(Debug)]
pub enum FunctionParam<'a> {
    /// key: value
    KeyValue(&'a str, Value<'a>),
    /// value (only)
    OnlyValue(Value<'a>),
}

impl<'a> From<Value<'a>> for FunctionParam<'a> {
    fn from(v: Value<'a>) -> Self {
        Self::OnlyValue(v)
    }
}

impl<'a> From<&'a str> for FunctionParam<'a> {
    fn from(v: &'a str) -> Self {
        Self::OnlyValue(Value::Text(Text(v)))
    }
}

impl<'a, T> From<Constant<T>> for FunctionParam<'a>
where
    T: fmt::Display + 'a,
{
    fn from(v: Constant<T>) -> Self {
        Self::OnlyValue(Value::Constant(Constant::new_no_validate(Box::new(v))))
    }
}

impl<'a, T> From<(&'a str, T)> for FunctionParam<'a>
where
    T: Into<Value<'a>>,
{
    fn from(kv: (&'a str, T)) -> Self {
        Self::KeyValue(kv.0, kv.1.into())
    }
}

impl<'a> fmt::Display for FunctionParam<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionParam::KeyValue(k, v) => {
                write!(f, "{k}: {v}")
            }
            FunctionParam::OnlyValue(v) => v.fmt(f),
        }
    }
}

/// Represents a function value in the PSL.
#[derive(Debug)]
pub struct Function<'a> {
    name: Constant<Cow<'a, str>>,
    params: Vec<FunctionParam<'a>>,
    render_empty_parentheses: bool,
}

impl<'a> Function<'a> {
    /// Creates a plain function with no parameters.
    pub fn new(name: &'a str) -> Self {
        match Constant::new(name) {
            Ok(name) => {
                let params = Vec::new();

                Self {
                    name,
                    params,
                    render_empty_parentheses: false,
                }
            }
            // Will render `sanitized(map: "original")`
            Err(ConstantNameValidationError::WasSanitized { sanitized }) => {
                let mut fun = Self {
                    name: sanitized,
                    params: Vec::new(),
                    render_empty_parentheses: false,
                };

                fun.push_param(("map", Text(name)));
                fun
            }
            // We just generate an invalid function in this case. It
            // will error in the validation.
            Err(ConstantNameValidationError::SanitizedEmpty) => {
                let mut fun = Self {
                    name: Constant::new_no_validate(Cow::Borrowed(name)),
                    params: Vec::new(),
                    render_empty_parentheses: false,
                };

                fun.push_param(("map", Text(name)));
                fun
            }
            // Interesting if this ever happens... Blame me in a zoom call if we
            // hit this.
            Err(ConstantNameValidationError::OriginalEmpty) => {
                let mut fun = Self {
                    name: Constant::new_no_validate(Cow::Borrowed("emptyValue")),
                    params: Vec::new(),
                    render_empty_parentheses: false,
                };

                fun.push_param(("map", Text(name)));
                fun
            }
        }
    }

    /// Add a new parameter to the function. If no parameters are
    /// added, the parentheses are not rendered.
    pub fn push_param(&mut self, param: impl Into<FunctionParam<'a>>) {
        self.params.push(param.into());
    }

    pub(crate) fn render_empty_parentheses(&mut self) {
        self.render_empty_parentheses = true;
    }
}

impl<'a> fmt::Display for Function<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)?;

        if !self.params.is_empty() || self.render_empty_parentheses {
            f.write_str("(")?;
        }

        if !self.params.is_empty() {
            for (i, param) in self.params.iter().enumerate() {
                param.fmt(f)?;

                if i < self.params.len() - 1 {
                    f.write_str(", ")?;
                }
            }
        }

        if !self.params.is_empty() || self.render_empty_parentheses {
            f.write_str(")")?;
        }

        Ok(())
    }
}
