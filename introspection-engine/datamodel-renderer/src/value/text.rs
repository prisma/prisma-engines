use std::fmt;

use psl::PreviewFeature;

/// Represents a string value in the PSL.
#[derive(Debug, Clone, Copy)]
pub struct Text<T: fmt::Display>(pub T);

impl<'a> fmt::Display for Text<&'a str> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&psl::schema_ast::string_literal(self.0), f)
    }
}

impl fmt::Display for Text<PreviewFeature> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;
        self.0.fmt(f)?;
        f.write_str("\"")?;

        Ok(())
    }
}
