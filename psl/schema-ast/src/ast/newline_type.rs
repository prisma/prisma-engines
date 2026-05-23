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
    /// The first newline that appears in the input wins: if it is preceded by a
    /// carriage return we report `Windows` (CRLF), otherwise `Unix` (LF). When
    /// the input contains no newline at all, the default (`Unix`) is returned.
    /// This mirrors the maintainer's guidance on prisma/prisma#8548: mixed-ending
    /// inputs fall through to the LF default rather than guessing.
    pub fn detect(input: &str) -> NewlineType {
        let bytes = input.as_bytes();
        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                if i > 0 && bytes[i - 1] == b'\r' {
                    return NewlineType::Windows;
                }
                return NewlineType::Unix;
            }
        }
        NewlineType::Unix
    }
}
