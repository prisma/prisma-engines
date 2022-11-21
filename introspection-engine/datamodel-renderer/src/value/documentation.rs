use std::{borrow::Cow, fmt};

/// A documentation block on top of an item in the PSL.
#[derive(Debug)]
pub struct Documentation<'a>(pub(crate) Cow<'a, str>);

impl<'a> fmt::Display for Documentation<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in self.0.split('\n') {
            f.write_str("///")?;

            if !line.is_empty() {
                f.write_str(" ")?;
            }

            f.write_str(line)?;
            f.write_str("\n")?;
        }

        Ok(())
    }
}
