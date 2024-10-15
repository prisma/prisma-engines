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
        self.inner.exec(message).map(|matches| {
            // We keep the same number of captures as the number of groups in the regex pattern,
            // but we guarantee that the captures are always strings.
            matches
                .iter()
                .map(|match_value| match_value.try_into().ok().unwrap_or_default())
                .collect()
        })
    }

    fn test(&self, input: &str) -> bool {
        self.inner.test(input)
    }
}
