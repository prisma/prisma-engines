use serde::Serialize;
use std::fmt;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderFormat {
    pub prefix: &'static str,
    pub has_numbering: bool,
}

impl PlaceholderFormat {
    pub fn write(&self, sql: &mut String, placeholder_number: &mut i32) {
        sql.push_str(self.prefix);
        if self.has_numbering {
            sql.push_str(placeholder_number.to_string().as_str());
            *placeholder_number += 1;
        }
    }

    pub fn fmt(&self, f: &mut fmt::Formatter<'_>, placeholder_number: &mut i32) -> fmt::Result {
        f.write_str(self.prefix)?;
        if self.has_numbering {
            f.write_str(placeholder_number.to_string().as_str())?;
            *placeholder_number += 1;
        }
        Ok(())
    }
}
