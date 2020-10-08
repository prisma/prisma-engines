use super::*;
use crate::error::DatamodelError;

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

    pub fn validate(&self, schema_item: &str) -> Result<(), DatamodelError> {
        if self.name.is_empty() {
            Err(DatamodelError::new_validation_error(
                &format!("The name of a {} must not be empty.", schema_item),
                self.span,
            ))
        } else if self.name.chars().next().unwrap().is_numeric() {
            Err(DatamodelError::new_validation_error(
                &format!("The name of a {} must not start with a number.", schema_item),
                self.span,
            ))
        } else if self.name.contains('-') {
            Err(DatamodelError::new_validation_error(
                &format!("The character `-` is not allowed in {} names.", schema_item),
                self.span,
            ))
        } else {
            Ok(())
        }
    }
}

impl WithSpan for Identifier {
    fn span(&self) -> &Span {
        &self.span
    }
}
