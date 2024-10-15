use derive_more::Display;

#[derive(Debug, Display)]
#[display(fmt = "Regular expression error: {message}")]
pub struct RegExpError {
    pub message: String,
}

impl std::error::Error for RegExpError {}

/// Flag modifiers for regular expressions.
#[enumflags2::bitflags]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum RegExpFlags {
    IgnoreCase = 0b0001,
    Multiline = 0b0010,
}

impl RegExpFlags {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IgnoreCase => "i",
            Self::Multiline => "m",
        }
    }
}

pub trait RegExpCompat {
    /// Searches for the first match of this regex in the haystack given, and if found,
    /// returns not only the overall match but also the matches of each capture group in the regex.
    /// If no match is found, then None is returned.
    fn captures(&self, message: &str) -> Option<Vec<String>>;

    /// Tests if the regex matches the input string.
    fn test(&self, message: &str) -> bool;
}
