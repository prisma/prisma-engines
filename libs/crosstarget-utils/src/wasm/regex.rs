use js_sys::RegExp as JSRegExp;

use crate::common::{RegExpError, RegExpFlags};

pub struct RegExp {
    inner: JSRegExp,
}

impl RegExp {
    pub fn new(pattern: &str, flags: Vec<RegExpFlags>) -> Result<Self, RegExpError> {
        let mut flags_as_str: String = flags.into_iter().map(|flag| String::from(flag)).collect();

        // Global flag is implied in `regex::Regex`, so we match that behavior for consistency.
        flags_as_str.push('g');

        Ok(Self {
            inner: JSRegExp::new(pattern, &flags_as_str),
        })
    }

    /// Searches for the first match of this regex in the haystack given, and if found,
    /// returns not only the overall match but also the matches of each capture group in the regex.
    /// If no match is found, then None is returned.
    pub fn captures(&self, message: &str) -> Option<Vec<String>> {
        let matches = self.inner.exec(message);
        matches.map(|matches| {
            let mut captures = Vec::new();
            for i in 0..matches.length() {
                captures.push(matches.get(i).as_string().unwrap());
            }
            captures
        })
    }

    /// Tests if the regex matches the input string.
    pub fn test(&self, input: &str) -> bool {
        self.inner.test(input)
    }
}
