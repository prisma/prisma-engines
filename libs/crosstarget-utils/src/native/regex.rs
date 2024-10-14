use regex::{Regex as NativeRegex, RegexBuilder};

use crate::common::{RegExpError, RegExpFlags};

pub struct RegExp {
    inner: NativeRegex,
}

impl RegExp {
    pub fn new(pattern: &str, flags: Vec<RegExpFlags>) -> Result<Self, RegExpError> {
        let mut builder = RegexBuilder::new(pattern);

        if flags.contains(&RegExpFlags::Multiline) {
            builder.multi_line(true);
        }

        if flags.contains(&RegExpFlags::IgnoreCase) {
            builder.case_insensitive(true);
        }

        let inner = builder.build().map_err(|e| RegExpError { message: e.to_string() })?;

        Ok(Self { inner })
    }

    /// Searches for the first match of this regex in the haystack given, and if found,
    /// returns not only the overall match but also the matches of each capture group in the regex.
    /// If no match is found, then None is returned.
    pub fn captures(&self, message: &str) -> Option<Vec<String>> {
        self.inner.captures(message).map(|captures| {
            captures
                .iter()
                .flat_map(|capture| capture.map(|cap| cap.as_str().to_string()))
                .collect()
        })
    }

    /// Tests if the regex matches the input string.
    pub fn test(&self, message: &str) -> bool {
        self.inner.is_match(message)
    }
}
