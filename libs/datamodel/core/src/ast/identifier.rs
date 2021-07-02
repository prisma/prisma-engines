use super::*;
use crate::diagnostics::{DatamodelError, Diagnostics};

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

impl Identifier {
    pub fn new(name: &str) -> Identifier {
        Identifier {
            name: String::from(name),
            span: Span::empty(),
        }
    }

    pub fn validate(&self, schema_item: &str, diagnostics: &mut Diagnostics) {
        if self.name.is_empty() {
            diagnostics.push_error(DatamodelError::new_validation_error(
                &format!("The name of a {} must not be empty.", schema_item),
                self.span,
            ))
        } else if self.name.chars().next().unwrap().is_numeric() {
            diagnostics.push_error(DatamodelError::new_validation_error(
                &format!("The name of a {} must not start with a number.", schema_item),
                self.span,
            ))
        } else if self.name.contains('-') {
            diagnostics.push_error(DatamodelError::new_validation_error(
                &format!("The character `-` is not allowed in {} names.", schema_item),
                self.span,
            ))
        }
    }
}

impl WithSpan for Identifier {
    fn span(&self) -> &Span {
        &self.span
    }
}
