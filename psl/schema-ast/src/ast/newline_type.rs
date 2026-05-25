use std::fmt;

/// Defines the newline type of a PSL block.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum NewlineType {
    /// `\\n`
    #[default]
    Unix,
    /// `\\r\\n`
    Windows,
}

impl fmt::Display for NewlineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl AsRef<str> for NewlineType {
    fn as_ref(&self) -> &str {
        match self {
            NewlineType::Unix => "\n",
            NewlineType::Windows => "\r\n",
        }
    }
}

impl NewlineType {
    /// Detect the line-ending style used in the given input string.
    ///
    /// Scans the full input: if any bare `\n` (not preceded by `\r`) appears
    /// the result is `Unix` (LF), even when other newlines in the same input
    /// are CRLF. Only an input whose every newline is CRLF returns `Windows`.
    /// An input with no newline at all returns the default (`Unix`).
    ///
    /// This mirrors the maintainer's guidance on prisma/prisma#8548: mixed-ending
    /// inputs fall through to the LF default rather than guessing, regardless
    /// of which style appears first.
    pub fn detect(input: &str) -> NewlineType {
        let bytes = input.as_bytes();
        let mut saw_crlf = false;
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                if i > 0 && bytes[i - 1] == b'\r' {
                    saw_crlf = true;
                } else {
                    // A bare LF anywhere in the input forces LF output.
                    return NewlineType::Unix;
                }
            }
        }
        if saw_crlf {
            NewlineType::Windows
        } else {
            NewlineType::Unix
        }
    }
}
