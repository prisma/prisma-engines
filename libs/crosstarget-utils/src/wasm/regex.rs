use enumflags2::BitFlags;
use js_sys::RegExp as JSRegExp;

use crate::common::regex::{RegExpCompat, RegExpError, RegExpFlags};

pub struct RegExp {
    inner: JSRegExp,
}

impl RegExp {
    pub fn new(pattern: &str, flags: BitFlags<RegExpFlags>) -> Result<Self, RegExpError> {
        let mut flags: String = flags.into_iter().map(|flag| flag.as_str()).collect();

        // Global flag is implied in `regex::Regex`, so we match that behavior for consistency.
        flags.push('g');

        Ok(Self {
            inner: JSRegExp::new(pattern, &flags),
        })
    }
}

impl RegExpCompat for RegExp {
    fn captures(&self, message: &str) -> Option<Vec<String>> {
        let matches = self.inner.exec(message);
        matches.map(|matches| {
            let mut captures = Vec::new();
            for i in 0..matches.length() {
                let match_value = matches.get(i);

                // `match_value` may be `undefined`.
                if match_value.is_string() {
                    captures.push(match_value.as_string().unwrap());
                }
            }
            captures
        })
    }

    fn test(&self, input: &str) -> bool {
        self.inner.test(input)
    }
}
