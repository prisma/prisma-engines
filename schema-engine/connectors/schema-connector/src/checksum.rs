//! Checksums of migration scripts are used in various parts of the migration
//! engine to ensure integrity. This module contains common logic that should be
//! used everywhere for consistency.

/// Compute the checksum for a new migration script, and render it formatted to
/// a human readable string.
pub(crate) fn render_checksum(script: &str) -> String {
    compute_checksum(script).format_checksum()
}

/// Returns whether a migration script matches an existing checksum.
pub(crate) fn script_matches_checksum(script: &str, checksum: &str) -> bool {
    use std::iter::{once, once_with};

    // Checksum with potentially different line endings, so checksums will match
    // between Unix-like systems and Windows.
    //
    // This is necessary because git messes with line endings. For background
    // information, read
    // https://web.archive.org/web/20150912185006/http://adaptivepatchwork.com:80/2012/03/01/mind-the-end-of-your-line/
    let mut script_checksums = once(compute_checksum(script))
        .chain(once_with(|| compute_checksum(&script.replace("\r\n", "\n"))))
        .chain(once_with(|| compute_checksum(&script.replace('\n', "\r\n"))));

    script_checksums.any(|script_checksum| {
        // Due to an omission in a previous version of the schema engine,
        // some migrations tables will have old migrations with checksum strings
        // that have not been zero-padded.
        //
        // Corresponding issue:
        // https://github.com/prisma/prisma-engines/issues/1887
        let script_checksum_str = if !checksum.is_empty() && checksum.len() != CHECKSUM_STR_LEN {
            script_checksum.format_checksum_old()
        } else {
            script_checksum.format_checksum()
        };

        script_checksum_str == checksum
    })
}

/// Checksumming implementation. This should be the single place where we do this.
fn compute_checksum(script: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(script);
    hasher.finalize().into()
}

/// The length (in bytes, or equivalently ascii characters) of the checksum
/// strings.
const CHECKSUM_STR_LEN: usize = 64;

/// Format a checksum to a hexadecimal string. This is used to checksum
/// migration scripts with Sha256.
trait FormatChecksum {
    /// Format a checksum to a hexadecimal string.
    fn format_checksum(&self) -> String;
    /// Obsolete checksum method, should only be used for compatibility.
    fn format_checksum_old(&self) -> String;
}

impl FormatChecksum for [u8; 32] {
    fn format_checksum(&self) -> String {
        use std::fmt::Write as _;

        let mut checksum_string = String::with_capacity(32 * 2);

        for byte in self {
            write!(checksum_string, "{byte:02x}").unwrap();
        }

        assert_eq!(checksum_string.len(), CHECKSUM_STR_LEN);

        checksum_string
    }

    // Due to an omission in a previous version of the schema engine,
    // some migrations tables will have old migrations with checksum strings
    // that have not been zero-padded.
    //
    // Corresponding issue:
    // https://github.com/prisma/prisma-engines/issues/1887
    fn format_checksum_old(&self) -> String {
        use std::fmt::Write as _;

        let mut checksum_string = String::with_capacity(32 * 2);

        for byte in self {
            write!(checksum_string, "{byte:x}").unwrap();
        }

        checksum_string
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_checksum_does_not_strip_zeros() {
        assert_eq!(
            render_checksum("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(render_checksum("abcd").len(), CHECKSUM_STR_LEN);
    }

    #[test]
    fn script_matches_checksum_is_line_ending_agnostic() {
        let scripts = &[
            &["ab\ncd\nef\ngh\rab", "ab\r\ncd\r\nef\r\ngh\rab"],
            &["ab\ncd\nef\ngh\rab\n", "ab\r\ncd\r\nef\r\ngh\rab\r\n"],
        ];

        // for loops go brrrrrrrrr
        for scripts in scripts {
            for script in *scripts {
                for other_script in *scripts {
                    assert!(script_matches_checksum(script, &render_checksum(other_script)),);
                }
            }
        }
    }

    #[test]
    fn script_matches_checksum_negative() {
        assert!(!script_matches_checksum("abc", &render_checksum("abcd")));
        assert!(!script_matches_checksum("abc\n", &render_checksum("abc")));
    }
}
