//! Borrowed from https://docs.rs/common-path
use std::path::{Path, PathBuf};

/// Find the common prefix, if any, between any number of paths
pub fn common_path_all<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Option<PathBuf> {
    let mut path_iter = paths.into_iter();
    let mut result = path_iter.next()?.to_path_buf();
    for path in path_iter {
        if let Some(r) = common_path(&result, path) {
            result = r;
        } else {
            return None;
        }
    }
    Some(result.to_path_buf())
}

/// Find the common prefix, if any, between 2 paths
pub fn common_path<P, Q>(one: P, two: Q) -> Option<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let one = one.as_ref();
    let two = two.as_ref();
    let one = one.components();
    let two = two.components();
    let mut final_path = PathBuf::new();
    let mut found = false;
    let paths = one.zip(two);
    for (l, r) in paths {
        if l == r {
            final_path.push(l.as_os_str());
            found = true;
        } else {
            break;
        }
    }
    if found {
        Some(final_path)
    } else {
        None
    }
}
