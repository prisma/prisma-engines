//! Checksums of migration scripts are used in various parts of the migration
//! engine to ensure integrity. This module contains common logic that should be
//! used everywhere for consistency.

use sha2::{Digest, Sha256};

/// Compute the checksum for a migration script, and render it formatted to a
/// human readable string.
pub(crate) fn render_checksum(script: &str) -> String {
    let mut hasher = Sha256::new();

    // Normalize line endings so checksums are identical between unix-like
    // systems and windows.
    //
    // This is necessary because git messes with line endings. For background
    // information, read
    // https://web.archive.org/web/20150912185006/http://adaptivepatchwork.com:80/2012/03/01/mind-the-end-of-your-line/
    if script.contains("\r\n") {}

    hasher.update(script.as_bytes());
    let checksum: [u8; 32] = hasher.finalize().into();
    checksum.format_checksum()
}

/// Returns whether a migration script matches an existing checksum.
pub(crate) fn script_matches_checksum(script: &str, checksum: &str) -> bool {
    let script_checksum = compute_checksum(script);

    // Due to an omission in a previous version of the migration engine,
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
}

fn compute_checksum(script: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&script);
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
            write!(checksum_string, "{:02x}", byte).unwrap();
        }

        assert_eq!(checksum_string.len(), CHECKSUM_STR_LEN);

        checksum_string
    }

    // Due to an omission in a previous version of the migration engine,
    // some migrations tables will have old migrations with checksum strings
    // that have not been zero-padded.
    //
    // Corresponding issue:
    // https://github.com/prisma/prisma-engines/issues/1887
    fn format_checksum_old(&self) -> String {
        use std::fmt::Write as _;

        let mut checksum_string = String::with_capacity(32 * 2);

        for byte in self {
            write!(checksum_string, "{:x}", byte).unwrap();
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
}
