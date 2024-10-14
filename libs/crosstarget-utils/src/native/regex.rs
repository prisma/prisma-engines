use enumflags2::BitFlags;
use regex::{Regex as NativeRegex, RegexBuilder};

use crate::common::regex::{RegExpCompat, RegExpError, RegExpFlags};

pub struct RegExp {
    inner: NativeRegex,
}

impl RegExp {
    pub fn new(pattern: &str, flags: BitFlags<RegExpFlags>) -> Result<Self, RegExpError> {
        let mut builder = RegexBuilder::new(pattern);

        if flags.contains(RegExpFlags::Multiline) {
            builder.multi_line(true);
        }

        if flags.contains(RegExpFlags::IgnoreCase) {
            builder.case_insensitive(true);
        }

        let inner = builder.build().map_err(|e| RegExpError { message: e.to_string() })?;

        Ok(Self { inner })
    }
}

impl RegExpCompat for RegExp {
    fn captures(&self, message: &str) -> Option<Vec<String>> {
        self.inner.captures(message).map(|captures| {
            captures
                .iter()
                .flat_map(|capture| capture.map(|cap| cap.as_str().to_owned()))
                .collect()
        })
    }

    fn test(&self, message: &str) -> bool {
        self.inner.is_match(message)
    }
}
