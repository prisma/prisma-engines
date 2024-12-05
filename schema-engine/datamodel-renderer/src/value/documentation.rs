use std::{borrow::Cow, fmt};

/// A documentation block on top of an item in the PSL.
#[derive(Debug)]
pub struct Documentation<'a>(pub(crate) Cow<'a, str>);

impl<'a> Documentation<'a> {
    pub(crate) fn push(&mut self, docs: Cow<'a, str>) {
        match &mut self.0 {
            Cow::Owned(d) => {
                d.push('\n');
                d.push_str(docs.as_ref());
            }
            Cow::Borrowed(existing) => {
                self.0 = Cow::Owned(format!("{existing}\n{}", docs));
            }
        }
    }
}

impl fmt::Display for Documentation<'_> {
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
