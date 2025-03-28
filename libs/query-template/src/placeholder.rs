use serde::Serialize;
use std::fmt;
use std::fmt::Write;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderFormat {
    pub prefix: &'static str,
    pub has_numbering: bool,
}

impl PlaceholderFormat {
    pub fn write<W: Write>(&self, writer: &mut W, placeholder_number: &mut i32) -> fmt::Result {
        writer.write_str(self.prefix)?;
        if self.has_numbering {
            write!(writer, "{placeholder_number}")?;
            *placeholder_number += 1;
        }
        Ok(())
    }
}
